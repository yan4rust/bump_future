use std::sync::Arc;

use bumpalo::Bump;
use crossbeam_queue::ArrayQueue;

use super::RecycleableBump;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub pool_capacity: usize,
    pub bump_capacity: usize,
}

pub struct BumpPool {
    pool: Arc<ArrayQueue<Bump>>,
    bump_capacity: usize,
}
impl BumpPool {
    pub fn new(pool_capacity: usize, bump_capacity: usize) -> Self {
        let pool = ArrayQueue::new(pool_capacity);
        for _idx in 0..pool_capacity {
            let _ = pool.push(Bump::with_capacity(bump_capacity));
        }
        Self {
            pool: Arc::new(pool),
            bump_capacity,
        }
    }
    /// pool cappacity
    pub fn capacity(&self) -> usize {
        self.pool.capacity()
    }
    /// how many Bump instance in pool
    pub fn len(&self) -> usize {
        self.pool.len()
    }
}
impl BumpPool {
    /// take a Bump instance from pool,and return RecycleableBump
    /// when no Bump,it will create a new Bump instance。
    /// when RecycleableBump dropped, it will reset Bump and push back into the pool
    /// with the pool,we can resuse pre allocated memory in Bump instance
    /// and reduce the memory allocation system call
    pub fn take(&self) -> RecycleableBump {
        let pool = Arc::downgrade(&self.pool);
        let bump = self
            .pool
            .pop()
            .or_else(|| Some(Bump::with_capacity(self.bump_capacity)))
            .unwrap();
        RecycleableBump {
            bump: Some(bump),
            pool,
        }
    }
}
