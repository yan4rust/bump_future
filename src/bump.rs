use std::{ops::Deref, sync::Weak};

use bumpalo::Bump;
use crossbeam_queue::ArrayQueue;
use tokio::sync::mpsc;


pub mod pool;



/// Bump usage reference manager
pub struct BumpRefMgr {
    rx: mpsc::Receiver<()>,
    tx: mpsc::Sender<()>,
}
impl Default for BumpRefMgr {
    fn default() -> Self {
        Self::new()
    }
}

impl BumpRefMgr {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        Self { rx, tx }
    }
    pub fn new_ref(&self) -> BumpRef {
        BumpRef {
            _tx: self.tx.clone(),
        }
    }
    /// when Future resolved,it means all BumpRef dropped,
    pub async fn wait_no_ref(mut self) {
        drop(self.tx);
        self.rx.recv().await;
    }
}

/// Bump usage reference object
/// any object stored in Bump must hold a BumpRef to prevent the Bump from released
pub struct BumpRef {
    _tx: mpsc::Sender<()>,
}

/// when dropped,Bump instance will be reset and send back to pool
pub struct RecycleableBump {
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
            let _ = pool.push(bump);
        }
    }
}
