# Type Erased Future Stored In [Bump](https://docs.rs/bumpalo/latest/bumpalo/)
Most time we use async function or async block to implement Future logic, the type of Future is unknown until compile.

But sometimes we need named Future type. For example , hyper [Service](https://docs.rs/hyper/latest/hyper/service/trait.Service.html) trait require name the Future type.

One solution is to use [BoxFuture](https://docs.rs/futures/latest/futures/future/type.BoxFuture.html), for http request processing, it will frequently allocate and release
memory on heap.

With [BumpFuture], it will use a pool of [Bump](https://docs.rs/bumpalo/latest/bumpalo/struct.Bump.html) instance for storage, for every request processing ,
take a Bump from pool and use it as storage for all Future create for this request,
after request processed, the Bump will be reset and release back to pool. thus we can reduce memory allocation syscall.

It seems that about 5%-10% improvements of Req/Sec when use [BumpFuture].

# Examples
```
use bump_future::bump::pool::PoolConfig;
use bump_future::future::BumpFutureExt;
use bump_future::alloc_mod;
use bump_future::tokio;
use std::time::Duration;

alloc_mod!(bump_alloc);

#[tokio::main]
async fn main() {
    let conf = PoolConfig {
        pool_capacity: 8,
        bump_capacity: 1024,
    };
    let _ = bump_alloc::init(conf);
    
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
    let rslt = fut.await;
    assert_eq!(rslt, 32);
}
```
For a real hyper server example, see examples dir.