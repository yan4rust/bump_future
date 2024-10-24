#![allow(warnings)]
//! a sample hyper server used for benchmark test
//! it seems that about 5%-10% improvements of Req/Sec when use BumpFuture
//! test steps: 
//! 1. cargo install rewrk --git https://github.com/ChillFish8/rewrk.git
//! 2. cargo build --release --examples
//! 3. start with BumpFuture, "nohup ./target/release/examples/hyper_server --bump >/dev/null 2>&1 &" or start with BoxFuture, "nohup ./target/release/examples/hyper_server >/dev/null 2>&1 &"
//! 4. run rewrk , "rewrk -c 256 -t 2 -d 20s -h http://127.0.0.1:3000"

use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

use bump_future::bump::pool::PoolConfig;
use bump_future::future::{BumpFuture, BumpFutureExt};
use bytes::Bytes;
use clap::Parser;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::{service_fn, Service};
use hyper::{Request, Response};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use tokio_util::either::Either;

bump_future::alloc_mod!(bump_alloc);

type ServiceResult = Result<Response<Full<Bytes>>, Infallible>;

/// serve request with either BoxService or BumpService
fn serve_request(
    bump: bool,
    req: Request<Incoming>,
) -> impl Future<Output = ServiceResult> + Send + 'static {
    if bump {
        let msg = Msg::text("Hello World! This message from BumpFuture");
        let fut = async move {
            let svc = BumpService(msg);
            let rslt = svc.call(req).await;
            rslt
        };
        // every Request processing use a Bump and released after processed
        return Either::Left(bump_alloc::set_bump(fut));
    } else {
        let msg = Msg::text("Hello World! This message from BoxFuture");
        let fut = async move {
            let svc = BoxService(msg);
            let rslt = svc.call(req).await;
            rslt
        };
        return Either::Right(fut);
    }
}

struct Msg(pub Bytes);

impl Msg {
    pub fn text(msg: &str) -> Self {
        return Self(Bytes::copy_from_slice(msg.as_bytes()));
    }
}

/// a Service return BoxFuture
struct BoxService(pub Msg);

impl Service<Request<Incoming>> for BoxService {
    type Response = Response<Full<Bytes>>;

    type Error = Infallible;

    type Future = Pin<Box<dyn Future<Output = ServiceResult> + Send + 'static>>;

    fn call(&self, _req: Request<Incoming>) -> Self::Future {
        let msg = self.0 .0.clone();
        let ret = Box::pin(async move { return Ok(Response::new(Full::new(msg))) });
        return ret;
    }
}

/// a Service return BumpFuture
struct BumpService(pub Msg);

impl Service<Request<Incoming>> for BumpService {
    type Response = Response<Full<Bytes>>;

    type Error = Infallible;

    type Future = BumpFuture<ServiceResult>;

    fn call(&self, _req: Request<Incoming>) -> Self::Future {
        let msg = self.0 .0.clone();
        let ret = bump_alloc::with_task_or_new(move |alloc| {
            let fut = async move { return Ok(Response::new(Full::new(msg))) }.bumped(alloc);
            fut
        });
        return ret;
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {

    // if use BumpFuture
    #[arg(short, long, default_value_t = false)]
    pub bump: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // pre allocate memory for BumFuture use
    let conf = PoolConfig {
        pool_capacity: 1024 * 100,
        bump_capacity: 1024,
    };
    bump_alloc::init(conf).unwrap();

    let cli = Cli::parse();
    let bump = cli.bump;

    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (tcp, _) = listener.accept().await?;
        let io = TokioIo::new(tcp);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req: Request<Incoming>| {
                        return serve_request(bump, req);
                    }),
                )
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
