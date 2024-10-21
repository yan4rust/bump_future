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

use bumpalo::Bump;
use std::{alloc::Layout, any::TypeId, num::NonZeroUsize, ptr::NonNull, sync::Arc};

use sptr::Strict;

use crate::{bump::BumpRef, util::addr_to_ptr};

pub(crate) unsafe fn drop_by_addr<T>(addr: NonZeroUsize) {
    let ptr: NonNull<T> = NonNull::dangling();
    let ptr = ptr.as_ptr().with_addr(addr.get());
    std::ptr::drop_in_place(ptr);
}

/// a smart pointer point to object stored in Bump
/// the safety depends on Bump not reset or droped while this object is still live
pub(crate) struct Inner {
    addr: Option<NonZeroUsize>,
    type_id: TypeId,
    drop_fn: unsafe fn(NonZeroUsize),
}
impl Inner {
    pub unsafe fn new<T>(bump: &Bump, inner: T) -> Self
    where
        T: 'static,
    {
        let layout = Layout::new::<T>();
        let ptr = bump.alloc_layout(layout.clone());
        let ptr = unsafe {
            let ptr = ptr.cast::<T>();
            ptr.as_ptr().write(inner);
            ptr
        };
        let addr = ptr.as_ptr().addr();
        return Self {
            addr: Some(NonZeroUsize::new(addr).expect("addr shoud not be zero")),
            type_id: TypeId::of::<T>(),
            drop_fn: drop_by_addr::<T>,
        };
    }

    /// check if this object is of type T
    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: 'static,
    {
        let tid = TypeId::of::<T>();
        return &tid == &self.type_id;
    }
    /// if this object is of type T, will return the reference of T
    /// otherwise will return None
    pub unsafe fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        if self.is::<T>() {
            let ptr: NonNull<T> = addr_to_ptr::<T>(self.addr.as_ref().take().unwrap().clone());
            // 指针转换为引用
            let inner = &*ptr.as_ptr();
            Some(inner)
        } else {
            None
        }
    }
    /// if this object is of type T, will return the mutable reference of T
    /// otherwise will return None
    pub unsafe fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        if self.is::<T>() {
            let ptr: NonNull<T> = addr_to_ptr::<T>(self.addr.as_ref().take().unwrap().clone());
            // 指针转换为引用
            let inner = &mut *ptr.as_ptr();
            Some(inner)
        } else {
            None
        }
    }
}
impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(addr) = self.addr.take() {
            unsafe { (self.drop_fn)(addr) };
        }
    }
}

/// a object stored in Bump
pub struct BumpObject {
    inner: Inner,
    bump_ref: BumpRef,
}
impl BumpObject {
    pub fn new(inner: Inner, bump_ref: BumpRef) -> Self {
        return Self { inner, bump_ref };
    }
}

/// like std Any, downcast BumpObject to concret type
pub trait BumpAny {
    fn is<T>(&self) -> bool
    where
        T: 'static;
    fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static;
    fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static;
}

impl BumpAny for BumpObject {
    fn is<T>(&self) -> bool
    where
        T: 'static,
    {
        let tid = TypeId::of::<T>();
        return &tid == &self.inner.type_id;
    }

    fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        unsafe {self.inner.downcast_ref::<T>()}
    }

    fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        unsafe {self.inner.downcast_mut::<T>()}
    }
}
