use crate::stack::{Node, Stack};

use core::mem::transmute;
use core::ops::{Deref, DerefMut, Drop};

#[derive(Debug)]
pub struct Box<T>
where
    T: 'static,
{
    pub(crate) node: &'static mut Node<T>,
}

impl<T> Box<T> {
    pub(crate) fn new(node: &'static mut Node<T>) -> Self {
        Self { node }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.node.data
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node.data
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        let stack: &Stack = unsafe { transmute(self.node.next) };
        stack.push(self.node);
    }
}
