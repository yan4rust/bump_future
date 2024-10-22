use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{alloc::BumpAlloc, obj::BumpObject, util::poll_future};

/// a type erased Future,stored in Bump
pub struct BumpFuture<O> {
    inner: BumpObject,
    poll_fn: fn(this: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O>,
    // Self is essentially a pointer,so is Unpin
    _p: PhantomData<dyn Future<Output = O> + Send + Unpin + 'static>,
}
impl<O> BumpFuture<O> {
    pub(crate) fn new(
        inner: BumpObject,
        poll_fn: fn(this: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O>,
    ) -> Self {
        return Self {
            inner,
            poll_fn,
            _p: PhantomData::default(),
        };
    }
}
impl<O> AsRef<BumpObject> for BumpFuture<O> {
    fn as_ref(&self) -> &BumpObject {
        return &self.inner;
    }
}
impl<O> AsMut<BumpObject> for BumpFuture<O> {
    fn as_mut(&mut self) -> &mut BumpObject {
        return &mut self.inner;
    }
}
impl<O> Future for BumpFuture<O> {
    type Output = O;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        return (self.poll_fn)(self, cx);
    }
}

pub trait BumpFutureExt<O> {
    /// take a BumpAlloc impl reference as input,and will convert self into BumpFuture
    fn bumped<T>(self, alloc: &T) -> BumpFuture<O>
    where
        T: BumpAlloc;
}
impl<F, O> BumpFutureExt<O> for F
where
    F: Future<Output = O> + Send + 'static,
{
    fn bumped<T>(self, alloc: &T) -> BumpFuture<O>
    where
        T: BumpAlloc,
    {
        let obj = alloc.alloc(self);
        let poll_fn = poll_future::<BumpFuture<O>, F>;
        return BumpFuture::new(obj, poll_fn);
    }
}

#[cfg(test)]
mod test {
    use crate::util::{check_send, check_sync, check_unpin};

    use super::BumpFuture;

    #[test]
    fn test_future_bounds() {
        //ensure BumpFuture is Send
        check_send::<BumpFuture<()>>();

        //ensure BumpFuture is Unpin
        check_unpin::<BumpFuture<()>>();

        // ensure BumpFuture is !Sync,following code should not compile
        // check_sync::<BumpFuture<()>>();
    }
}
