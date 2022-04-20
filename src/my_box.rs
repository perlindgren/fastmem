use crate::stack::{Node, Stack};

use crate::trace;
use core::mem::transmute;
use core::ops::{Deref, DerefMut, Drop};

#[derive(Debug)]
pub struct Box<T>
where
    T: 'static,
{
    pub node: &'static Node<T>,
}

impl<T> Box<T> {
    pub(crate) fn new(node: &'static Node<T>) -> Self {
        Self { node }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.node.ptr_as_ref()
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.node.ptr_as_mut_ref()
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        trace!("drop @box {:p}", self);
        trace!("drop node {}", self.node);
        trace!("drop next {:p}", self.node.next.as_ptr());

        let stack: &Stack = unsafe { transmute(self.node.next.get()) };

        stack.push(self.node);
    }
}

unsafe impl<T> Send for Box<T> where T: Send {}
