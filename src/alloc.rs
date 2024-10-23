use bumpalo::Bump;
use tokio::runtime::Handle;

use crate::{
    bump::{BumpRef, BumpRefMgr, RecycleableBump},
    obj::{BumpObject, UnsafeObject},
};

/// BumpObject alloc trait
pub trait BumpAlloc {
    /// alloc a BumpObject in the Bump managed
    fn alloc<T>(&self, val: T) -> BumpObject
    where
        T: Send + 'static;
}

/// alloc object in bump within async task
/// when dropped ,it will spawn a task to wait all BumpRef dropped.
/// when all BumpRef dropped,it will drop RecycleableBump,and the Bump will be
/// reset and push back to pool
pub struct TaskBumpAlloc {
    handle: Handle,
    bump: Option<RecycleableBump>,
    ref_mgr: Option<BumpRefMgr>,
}

impl TaskBumpAlloc {
    pub fn new(handle: Handle, bump: RecycleableBump) -> Self {
        return Self {
            handle,
            bump: Some(bump),
            ref_mgr: Some(BumpRefMgr::new()),
        };
    }
    #[inline]
    fn bump(&self) -> &Bump {
        self.bump.as_ref().unwrap()
    }
    #[inline]
    fn new_bump_ref(&self) -> BumpRef {
        self.ref_mgr.as_ref().unwrap().new_ref()
    }
}
impl BumpAlloc for TaskBumpAlloc {
    fn alloc<T>(&self, val: T) -> crate::obj::BumpObject
    where
        T: Send + 'static,
    {
        let inner = unsafe { UnsafeObject::new(self.bump(), val) };
        let bump_ref = self.new_bump_ref();
        return BumpObject::new(inner, bump_ref);
    }
}
impl Drop for TaskBumpAlloc {
    fn drop(&mut self) {
        let bump = self.bump.take().expect("should not be None");
        let ref_mgr = self.ref_mgr.take().expect("should not be None");
        self.handle.spawn(async move {
            ref_mgr.wait_no_ref().await;
            drop(bump);
        });
    }
}
