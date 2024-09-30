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
    alloc::Layout, any::TypeId, num::NonZeroUsize, ptr::NonNull,
    sync::Arc,
};

unsafe fn drop_by_addr<T>(addr: NonZeroUsize) {
    let ptr: NonNull<T> = NonNull::dangling().with_addr(addr);
    std::ptr::drop_in_place(ptr.as_ptr());
}

struct Inner {
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
        return Self {
            addr: Some(ptr.addr()),
            type_id: TypeId::of::<T>(),
            drop_fn: drop_by_addr::<T>,
        };
    }
}

pub struct BumpObject {
    bump: Arc<Bump>,
}
