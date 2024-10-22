use std::{ops::Deref, sync::Weak};

use bumpalo::Bump;
use crossbeam_queue::ArrayQueue;
use tokio::{runtime::Handle, sync::mpsc};

use crate::{
    alloc::BumpAlloc,
    obj::{BumpObject, Inner},
};

pub mod pool;

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
        let inner = unsafe { Inner::new(self.bump(), val) };
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

/// bump reference manager
pub struct BumpRefMgr {
    rx: mpsc::Receiver<()>,
    tx: mpsc::Sender<()>,
}
impl BumpRefMgr {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        return Self { rx, tx };
    }
    pub fn new_ref(&self) -> BumpRef {
        return BumpRef {
            tx: self.tx.clone(),
        };
    }
    /// when Future resolved,it means all BumpRef dropped,
    pub async fn wait_no_ref(mut self) {
        drop(self.tx);
        self.rx.recv().await;
    }
}

/// Bump usage reference object
/// any object stored in Bump must hold a BumpRef to prevent the Bump to be released
pub struct BumpRef {
    tx: mpsc::Sender<()>,
}

/// when dropped,Bump instance will be reset and send back to pool
pub(crate) struct RecycleableBump {
    bump: Option<Bump>,
    pool: Weak<ArrayQueue<Bump>>,
}
impl Deref for RecycleableBump {
    type Target = Bump;

    fn deref(&self) -> &Self::Target {
        return self.bump.as_ref().expect("should not be None");
    }
}
impl Drop for RecycleableBump {
    fn drop(&mut self) {
        let mut bump = self.bump.take().expect("should not be None");
        bump.reset();
        if let Some(pool) = self.pool.upgrade() {
            pool.push(bump);
        }
    }
}
