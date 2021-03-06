use crate::stack::Node;
use crate::Alloc;
use core::mem::size_of;
use core::ops::{Deref, DerefMut, Drop};

#[derive(Debug)]
pub struct Box<T>
where
    T: 'static,
{
    pub inner: &'static mut T,
    pub allocator: &'static Alloc,
    pub node: &'static mut Node,
}

impl<T> Box<T> {
    pub fn new(t: &'static mut T, allocator: &'static Alloc, node: &'static mut Node) -> Self {
        println!("New box, {}", size_of::<T>());

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
        println!("Dropping box");
        let size = size_of::<T>();
        println!("size {}", size);
        self.allocator.free(self);
    }
}
