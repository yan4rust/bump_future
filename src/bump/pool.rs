use std::sync::Arc;

use bumpalo::Bump;
use crossbeam_queue::ArrayQueue;

use super::RecycleableBump;

/// config for BumpPool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Max instance count of pool
    pub pool_capacity: usize,
    /// Capacity of Bump instance
    pub bump_capacity: usize,
}

/// Pool of Bump instance
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
    /// Pool cappacity
    pub fn capacity(&self) -> usize {
        self.pool.capacity()
    }
    /// How many Bump instance in pool
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}
impl BumpPool {
    /// Take a Bump instance from pool,and return RecycleableBump
    /// When no Bump instance in pool,it will create a new Bump instance。
    /// When RecycleableBump dropped, it will reset Bump and release back into the pool
    /// With the pool,we can resuse pre allocated memory in Bump instance and reduce the memory allocation syscall
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
