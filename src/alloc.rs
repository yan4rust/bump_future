use crate::obj::BumpObject;

/// BumpObject alloc trait
pub trait BumpAlloc {
    /// alloc a BumpObject in the Bump managed
    fn alloc<T>(&self, val: T) -> BumpObject
    where
        T: Send + 'static;
}
