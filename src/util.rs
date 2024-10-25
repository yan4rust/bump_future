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

//! unsafe util

use sptr::Strict;
use std::{
    future::Future,
    num::NonZeroUsize,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use crate::obj::{BumpAny, BumpObject};

#[inline]
pub(crate) unsafe fn drop_by_addr<T>(addr: NonZeroUsize) {
    let ptr: NonNull<T> = NonNull::dangling();
    let ptr = ptr.as_ptr().with_addr(addr.get());
    std::ptr::drop_in_place(ptr);
}

/// convert addr from ptr
#[inline]
pub(crate) unsafe fn addr_of_ptr<T>(ptr: NonNull<T>) -> NonZeroUsize {
    let addr = ptr.as_ptr().addr();
    NonZeroUsize::new(addr).expect("addr shoud not be zero")
}
/// convert addr to ptr
#[inline]
pub(crate) unsafe fn addr_to_ptr<T>(addr: NonZeroUsize) -> NonNull<T> {
    let ptr: NonNull<T> = NonNull::dangling();
    let ptr = ptr.as_ptr().with_addr(addr.get());
    NonNull::<T>::new(ptr).expect("ptr should not be null")
}

#[inline]
pub(crate) fn poll_future<B, F>(this: Pin<&mut B>, cx: &mut Context<'_>) -> Poll<F::Output>
where
    B: AsMut<BumpObject>,
    F: Future + 'static,
{
    return as_pin_mut::<B, F>(this).poll(cx);
}

/// help function to map Pin of BumpObject to the type it wrapps
#[inline]
pub(crate) fn as_pin_mut<B, S>(this: Pin<&mut B>) -> Pin<&mut S>
where
    B: AsMut<BumpObject>,
    S: 'static,
{
    unsafe {
        let ret = this.map_unchecked_mut(|this| {
            let obj = this.as_mut();
            let ret = obj.downcast_mut::<S>().expect("type mismatch");
            ret
        });
        ret
    }
}

// check a Future is Unpin,if not compile ,the Future is !Unpin
pub(crate) fn check_unpin_ref<T>(_fut: &T)
where
    T: Future + Unpin,
{
}
pub(crate) fn check_unpin<T>()
where
    T: Future + Unpin,
{
}
pub(crate) fn check_send<T>()
where
    T: Send,
{
}
pub(crate) fn check_sync<T>()
where
    T: Send + Sync,
{
}
