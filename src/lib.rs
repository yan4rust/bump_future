// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(dead_code)]
#![allow(unstable_name_collisions)]
pub mod alloc;
pub mod bump;
pub mod future;
pub mod obj;
pub(crate) mod util;

pub mod tokio {
    pub use tokio::*;
}
pub mod once_cell {
    pub use once_cell::*;
}

/// generate a mod to use BumpFuture
#[macro_export]
macro_rules! alloc_mod {
    ( $vis:vis $name:ident ) => {
        $vis mod $name {
            use std::future::Future;

            use $crate::alloc::TaskBumpAlloc;
            use $crate::once_cell::sync::{Lazy, OnceCell};
            use $crate::tokio::{
                runtime::Handle,
                task::futures::TaskLocalFuture,
                task_local,
            };

            use $crate::bump::{
                    pool::{BumpPool, PoolConfig},
                };
            static POOL_CONFIG: OnceCell<PoolConfig> = OnceCell::new();
            static POOL: Lazy<BumpPool> = Lazy::new(|| {
                let conf = POOL_CONFIG.get();
                assert!(conf.is_some(),"mod '{}' not init yet",stringify!($name));
                let conf = conf.unwrap();
                BumpPool::new(conf.pool_capacity, conf.bump_capacity)
            });
            task_local! {
                pub static TASK_ALLOC: TaskBumpAlloc;
            }

            /// init with config
            pub fn init(config: PoolConfig) -> Result<(), PoolConfig> {
                return POOL_CONFIG.set(config);
            }
            /// access the TaskBumpAlloc associate with the current task
            /// must call within async context otherwise will panic
            /// if no TaskBumpAlloc with current task,it will take one from pool
            pub fn with_task_or_new<F, R>(func: F) -> R
            where
                F: FnOnce(&TaskBumpAlloc) -> R,
            {
                let ret = TASK_ALLOC.try_with(|_alloc| ());
                match ret {
                    Ok(_) => {
                        return TASK_ALLOC.try_with(func).expect("should not be Err");
                    }
                    Err(_err) => {
                        let bump = POOL.take();
                        let alloc = TaskBumpAlloc::new(Handle::current(), bump);
                        return func(&alloc);
                    }
                }
            }
            /// return pool reference
            pub fn pool() -> &'static BumpPool {
                return &POOL;
            }
            /// access the TaskBumpAlloc associate with the current task
            /// must call within async context otherwise will panic
            /// if no TaskBumpAlloc with current task, it will return None
            pub fn with_task<F, R>(func: F) -> Option<R>
            where
                F: FnOnce(&TaskBumpAlloc) -> R,
            {
                let ret = TASK_ALLOC.try_with(|_alloc| ());
                match ret {
                    Ok(_) => {
                        let ret = TASK_ALLOC.try_with(func).expect("should not be Err");
                        return Some(ret);
                    }
                    Err(_err) => {
                        return None;
                    }
                }
            }

            /// set a TaskBumpAlloc with the Future input
            /// when the Future polled , it can access the TaskBumpAlloc
            pub fn set_bump<F>(fut: F) -> TaskLocalFuture<TaskBumpAlloc, F>
            where
                F: Future,
            {
                let bump = POOL.take();
                let alloc = TaskBumpAlloc::new(Handle::current(), bump);
                let fut = TASK_ALLOC.scope(alloc, fut);
                return fut;
            }
                }
            };
}

#[cfg(test)]
mod test {

    use std::time::Duration;

    use tokio::io::copy;

    use crate::alloc_mod;
    use crate::bump::pool::PoolConfig;
    use crate::future::BumpFutureExt;

    // generate a mod of name "bump_alloc"
    alloc_mod!(bump_alloc);

    #[tokio::test]
    async fn test_bump_future() {
        let conf = PoolConfig {
            pool_capacity: 8,
            bump_capacity: 1024,
        };
        let _ = bump_alloc::init(conf);

        //after init ,pool len should be 8
        assert_eq!(bump_alloc::pool().len(), 8);

        test_bump_future_simple().await;
        test_set_bump_multi_times().await;
        test_not_unpin_box().await;
        test_not_unpin_bump().await;
    }

    async fn test_bump_future_simple() {
        {
            let fut = bump_alloc::set_bump(async move {
                let fut = bump_alloc::with_task(|alloc| {
                    async move {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        32_u32
                    }
                    .bumped(alloc)
                });

                fut.unwrap().await
            });
            // after first use , pool len should be 7
            assert_eq!(bump_alloc::pool().len(), 7);
            let rslt = fut.await;
            assert_eq!(rslt, 32);
        }
        // wait Bump recycled, pool len should be 8
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(bump_alloc::pool().len(), 8);
    }
    async fn test_set_bump_multi_times() {
        {
            let fut = bump_alloc::set_bump(async move {
                let fut = bump_alloc::with_task(|alloc| {
                    async move {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        32_u32
                    }
                    .bumped(alloc)
                });

                fut.unwrap().await
            });
            // after first use , pool len should be 7
            assert_eq!(bump_alloc::pool().len(), 7);

            // set_bump second times,pool len should be 6
            let fut = bump_alloc::set_bump(fut);
            assert_eq!(bump_alloc::pool().len(), 6);

            let rslt = fut.await;
            assert_eq!(rslt, 32);
        }
        // wait Bump recycled, pool len should be 8
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(bump_alloc::pool().len(), 8);
    }

    // test future which is !Unpin with Box
    async fn test_not_unpin_box() {
        let fut1 = async move {
            let mut x = Vec::with_capacity(16);
            let mut input = &b"12345"[..];
            let count = copy(&mut input, &mut x).await.unwrap();
            assert_eq!(count, 5);
            assert_eq!(&x[0..5], &b"12345"[0..5]);
            123_u32
        };
        // above Future is !Unpin, following code will not compile
        // check_unpin(&fut1);

        let fut = Box::pin(fut1);
        let rslt = fut.await;
        assert_eq!(rslt, 123);
    }
    // test future which is !Unpin with BumpFuture
    async fn test_not_unpin_bump() {
        let fut1 = async move {
            let mut x = Vec::with_capacity(16);
            let mut input = &b"12345"[..];
            let count = copy(&mut input, &mut x).await.unwrap();
            assert_eq!(count, 5);
            assert_eq!(&x[0..5], &b"12345"[0..5]);
            123_u32
        };
        // above Future is !Unpin, following code will not compile
        // check_unpin(&fut1);

        let fut = bump_alloc::set_bump(async move {
            let fut = bump_alloc::with_task(|alloc| fut1.bumped(alloc));

            fut.unwrap().await
        });
        let rslt = fut.await;
        assert_eq!(rslt, 123);
    }
}
