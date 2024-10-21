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

#![allow(warnings)]
#![allow(unstable_name_collisions)]
pub mod alloc;
pub mod bump;
pub mod future;
pub mod obj;
pub mod util;

// test mod for user face api, it should use macro to generate the mod
pub(crate) mod api {
    use std::future::Future;

    use once_cell::sync::{Lazy, OnceCell};
    use tokio::{runtime::Handle, task::JoinHandle, task_local};

    use crate::{alloc::BumpAlloc, bump::{pool::{BumpPool, PoolConfig}, TaskBumpAlloc}};


    static POOL_CONFIG: OnceCell<PoolConfig> = OnceCell::new();
    static POOL:Lazy<BumpPool> = Lazy::new(||{
        let conf = POOL_CONFIG.get().expect("should init first");
        BumpPool::new(conf.pool_capacity, conf.bump_capacity)
    });
    task_local! {
        pub static TASK_ALLOC: TaskBumpAlloc;
    }

    /// init with config
    pub fn init(config: PoolConfig)->Result<(),PoolConfig> {
        return POOL_CONFIG.set(config);
    }
    /// access the TaskBumpAlloc associate with the current task
    /// must call within async context otherwise will panic
    /// if no TaskBumpAlloc with current task,it will take one from pool
    pub fn task_local_bump<F, R>(func: F) -> R
    where
        F: FnOnce(&TaskBumpAlloc) -> R,
    {
        let ret = TASK_ALLOC.try_with(|alloc|{
            ()
        });
        match ret {
            Ok(_)=>{
                return TASK_ALLOC.try_with(func).expect("should not be Err");
            }
            Err(_err)=>{
                let bump = POOL.take();
                let alloc = TaskBumpAlloc::new(Handle::current(), bump);
                return func(&alloc);
            }
        }
    }

    /// spawn a future and set a TaskBumpAlloc with this task  with the task
    /// code running in task can use function 'use_bump' to access it
    pub fn spawn<F>(fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let bump = POOL.take();
        let alloc = TaskBumpAlloc::new(Handle::current(), bump);
        let fut = TASK_ALLOC.scope(alloc, fut);
        return tokio::spawn(fut);
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{api, bump::pool::PoolConfig};
    use crate::future::BumpFutureExt;

    #[tokio::test]
    async fn test_bump_future() {
        let conf = PoolConfig {
            pool_capacity:8,
            bump_capacity:1024,
        };
        api::init(conf).unwrap();
        let rslt = api::spawn(async move {
            
            let fut = api::task_local_bump(|alloc|{
                let fut = async move {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    32 as u32
                }.bumped(alloc);
                fut
            });
            let rslt = fut.await;
            rslt
            
        }).await.unwrap();
        assert_eq!(rslt,32);
    }
}
