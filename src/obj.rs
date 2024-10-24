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
use std::{
    alloc::Layout, any::TypeId, cell::Cell, marker::PhantomData, num::NonZeroUsize, ptr::NonNull,
};

use sptr::Strict;

use crate::{
    bump::BumpRef,
    util::{addr_to_ptr, drop_by_addr},
};

/// a smart pointer point to object stored in Bump
/// it use TypeId to check when downcast in runtime, so only 'static type can be used as input
pub struct UnsafeObject {
    addr: Option<NonZeroUsize>,
    type_id: TypeId,
    drop_fn: unsafe fn(NonZeroUsize),
    // Self only require input type is Send, we must ensure Self is !sync,
    _p: PhantomData<Cell<()>>,
}
impl UnsafeObject {
    /// # Safety
    /// the safety depends on Bump used to create this object not reset or droped while this object is still live
    pub unsafe fn new<T>(bump: &Bump, inner: T) -> Self
    where
        T: Send + 'static,
    {
        let layout = Layout::new::<T>();
        let ptr = bump.alloc_layout(layout);
        let ptr = unsafe {
            let ptr = ptr.cast::<T>();
            ptr.as_ptr().write(inner);
            ptr
        };
        let addr = ptr.as_ptr().addr();
        Self {
            addr: Some(NonZeroUsize::new(addr).expect("addr shoud not be zero")),
            type_id: TypeId::of::<T>(),
            drop_fn: drop_by_addr::<T>,
            _p: PhantomData,
        }
    }

    /// check if this object is of type T
    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: 'static,
    {
        let tid = TypeId::of::<T>();
        tid == self.type_id
    }
    /// # Safety
    /// the safety depends on Bump used to create this object not reset or droped while this object is still live
    /// if this object is of type T, will return the reference of T
    /// otherwise will return None
    #[inline]
    pub unsafe fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        if self.is::<T>() {
            let ptr: NonNull<T> = addr_to_ptr::<T>(*self.addr.as_ref().unwrap());
            let inner = &*ptr.as_ptr();
            Some(inner)
        } else {
            None
        }
    }
    /// # Safety
    /// the safety depends on Bump used to create this object not reset or droped while this object is still live
    /// if this object is of type T, will return the mutable reference of T
    /// otherwise will return None
    #[inline]
    pub unsafe fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        if self.is::<T>() {
            let ptr: NonNull<T> = addr_to_ptr::<T>(*self.addr.as_ref().unwrap());
            // 指针转换为引用
            let inner = &mut *ptr.as_ptr();
            Some(inner)
        } else {
            None
        }
    }
}
impl Drop for UnsafeObject {
    fn drop(&mut self) {
        if let Some(addr) = self.addr.take() {
            unsafe { (self.drop_fn)(addr) };
        }
    }
}

/// a object stored in Bump
pub struct BumpObject {
    inner: UnsafeObject,
    _bump_ref: BumpRef,
}
impl BumpObject {
    pub fn new(inner: UnsafeObject, bump_ref: BumpRef) -> Self {
        Self {
            inner,
            _bump_ref: bump_ref,
        }
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
        self.inner.is::<T>()
    }

    fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        unsafe { self.inner.downcast_ref::<T>() }
    }

    fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        unsafe { self.inner.downcast_mut::<T>() }
    }
}

#[cfg(test)]
mod test {
    use crate::util::check_send;

    use super::UnsafeObject;

    #[test]
    fn test_inner_bounds() {
        // ensure UnsafeObject is Send
        check_send::<UnsafeObject>();
        // ensure UnsafeObject is !Sync
        // check_sync::<UnsafeObject>();
    }
}
