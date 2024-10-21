use std::{num::NonZeroUsize, ptr::NonNull};
use sptr::Strict;

/// get addr from ptr
#[inline]
pub unsafe fn addr_of_ptr<T>(ptr: NonNull<T>) -> NonZeroUsize {
    let addr = ptr.as_ptr().addr();
    NonZeroUsize::new(addr).expect("addr shoud not be zero")
}
/// convert addr to ptr
#[inline]
pub unsafe fn addr_to_ptr<T>(addr: NonZeroUsize) -> NonNull<T> {
    let ptr: NonNull<T> = NonNull::dangling();
    let ptr = ptr.as_ptr().with_addr(addr.get());
    NonNull::<T>::new(ptr).expect("ptr should not be null")
}