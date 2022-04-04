use core::cell::Cell;
use core::fmt;
use core::mem::transmute;
use core::ptr::NonNull;

// Next field first, so we have a stable location
//
// Safety:
// The `next` field is used as a raw pointer
// to refer to the next Node on the stack.
//
// The `data' field holds the allocated data.
//
// The alignment of the data:T according to T.

#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub(crate) struct Node<T> {
    pub(crate) next: Option<NonNull<usize>>,
    pub(crate) data: T,
}

impl<T> Node<T> {
    const fn new(data: T) -> Self {
        Node { next: None, data }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub(crate) head: Cell<Option<NonNull<usize>>>,
}

impl Stack {
    pub(crate) const fn new() -> Self {
        Stack {
            head: Cell::new(None),
        }
    }

    /// push a node to the top of the stack
    #[inline(always)]
    pub(crate) fn push<T>(&self, node: &mut Node<T>) {
        node.next = self.head.get();
        self.head.set(NonNull::new(unsafe { transmute(node) }));
    }

    /// pop a node from the top of the stack
    #[inline(always)]
    pub(crate) fn pop<T>(&self) -> Option<&mut Node<T>> {
        match self.head.get() {
            Some(node) => {
                // Safety: size_of<T> <= size of data for the node
                let node: &mut Node<T> = unsafe { transmute(node) };
                self.head.set(node.next);

                // Optionally: erase the tail of the top node to return
                // node.next = None;
                //
                // (`next` field will be set on push)
                Some(node)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::{align_of_val, size_of, size_of_val};

    #[test]
    fn test_size_list() {
        let l = Stack::new();
        assert_eq!(size_of_val(&l), 8);
        assert_eq!(size_of::<Stack>(), 8)
    }

    #[test]
    fn test_stack_nodes() {
        let stack = Stack::new();

        // println!("{}", stack);

        let mut n1 = Node::new(42);
        // println!("n1 {}", n1);

        let mut n2 = Node::new([1, 2, 3, 4]);
        // println!("n2 {:?}", n2);

        stack.push(&mut n1);
        // println!("{}", stack);

        stack.push(&mut n2);
        // println!("{}", stack);

        let new_n2: &mut Node<[i32; 4]> = stack.pop().unwrap();
        // println!("new_n2 {:?}", new_n2);

        let new_n1: &mut Node<i32> = stack.pop().unwrap();
        // println!("new_n1 {:?}", new_n1);

        assert_eq!(stack.head.get(), None);
    }

    use crate::my_box::Box;
    #[test]
    fn test_stack_box() {
        static mut N1: Node<i32> = Node::new(42);

        // println!("n1 {:?}", unsafe { &N1 });
        let b1 = Box::new(unsafe { &mut N1 });
        // println!("b1 {:?}", b1);

        // Will never be pushed back on stack
        // leaking!
        core::mem::forget(b1);
    }

    #[test]
    fn test_alignment() {
        #[repr(C, align(8))]
        struct A8(u32);

        let n = Node::new(A8(8));
        assert_eq!(align_of_val(&n.data), 8);
    }
}

// helpers
// Notice, seen a stack of nodes the `next` field has erased type
// We assume a ZST and never print the data field.
impl<T> fmt::Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.next {
            Some(node) => {
                let node: &Node<()> = unsafe { transmute(node) };
                core::fmt::write(f, format_args!("{}", node))
            }
            None => core::fmt::write(f, format_args!("None")),
        }
    }
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.head.get() {
            Some(node) => {
                let node: &Node<()> = unsafe { transmute(node) };
                core::fmt::write(f, format_args!("{}", node))
            }
            None => core::fmt::write(f, format_args!("None")),
        }
    }
}
