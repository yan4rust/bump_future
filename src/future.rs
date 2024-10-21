use std::{future::Future, marker::PhantomData};

use crate::alloc::BumpAlloc;

/// a type earsed Future,stored in Bump
pub struct BumpFuture<O> {
    _phantom: PhantomData<dyn Future<Output = O> + Send + 'static>,
}
impl<O> Future for BumpFuture<O> {
    type Output = O;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        unimplemented!()
    }
}

pub trait BumpFutureExt<O> {
    /// take a BumpAlloc impl reference as input,and will convert self into BumpFuture
    fn bumped<T>(self, alloc: &T) -> BumpFuture<O> where T:BumpAlloc;
}
impl<F, O> BumpFutureExt<O> for F
where
    F: Future<Output = O> + Send + 'static,
{
    fn bumped<T>(self, alloc: &T) -> BumpFuture<O> where T:BumpAlloc{
        unimplemented!()
    }
}
