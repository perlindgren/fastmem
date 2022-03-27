use crate::stack::Node;
use crate::Alloc;
use core::ops::{Deref, DerefMut, Drop};

#[derive(Debug)]
pub struct Box<T> {
    pub inner: T,
    pub allocator: &'static Alloc,
    pub node: &'static Node,
}

impl<T> Box<T> {
    pub const fn new(t: T, allocator: &'static Alloc, node: &'static Node) -> Self {
        Self {
            inner: t,
            allocator,
            node,
        }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        println!("Dropping HasDrop!");
        self.allocator.free(self);
    }
}