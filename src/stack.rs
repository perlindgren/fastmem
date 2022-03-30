use core::cell::Cell;
use core::fmt;
use core::fmt::Debug;
use core::fmt::Display;
use core::mem::transmute;
use core::ptr::NonNull;

// Next field first, so we can transmute any Node<T> to Node<()>
// (Assuming we never touch the data field)
#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Node<T> {
    pub next: Option<NonNull<usize>>,
    pub data: T,
}

impl<T> Node<T> {
    pub const fn new(data: T) -> Self {
        Node { next: None, data }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub head: Cell<Option<NonNull<usize>>>,
}

impl Stack {
    pub const fn new() -> Self {
        Stack {
            head: Cell::new(None),
        }
    }

    /// push a node to the top of the stack
    #[inline(always)]
    pub fn push<T>(&self, n: &mut Node<T>) {
        println!("push self @{:p}, node @{:p}", self, n);
        n.next = self.head.get();
        self.head.set(NonNull::new(unsafe { transmute(n) }));
    }

    /// pop a node from the top of the stack
    #[inline(always)]
    pub fn pop<T>(&self) -> Option<&mut Node<T>> {
        match self.head.get() {
            Some(node) => {
                let node: &mut Node<T> = unsafe { transmute(node) };
                self.head.set(node.next);

                // erase the tail of the top node to return
                node.next = None;
                Some(node)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::{size_of, size_of_val};

    #[test]
    fn test_size_list() {
        let l = Stack::new();
        assert_eq!(size_of_val(&l), 8);
        assert_eq!(size_of::<Stack>(), 8)
    }

    #[test]
    fn test_stack_nodes() {
        let stack = Stack::new();

        println!("{}", stack);

        let mut n1 = Node::new(42);
        println!("n1 {}", n1);

        let mut n2 = Node::new([1, 2, 3, 4]);
        println!("n2 {:?}", n2);
        // n1.next = unsafe { transmute(&n2) };

        println!("n2 {}", n2);

        stack.push(&mut n1);
        println!("{}", stack);

        stack.push(&mut n2);
        println!("{}", stack);

        let new_n2: &mut Node<[i32; 4]> = stack.pop().unwrap();
        println!("new_n2 {:?}", new_n2);

        let new_n1: &mut Node<i32> = stack.pop().unwrap();
        println!("new_n1 {:?}", new_n1);

        assert_eq!(stack.head.get(), None);
    }

    use crate::my_box::Box;
    #[test]
    fn test_stack_box() {
        static mut N1: Node<i32> = Node::new(42);
        let stack = Stack::new();

        println!("n1 {}", unsafe { &N1 });
        let b1 = Box::new(unsafe { &mut N1 });
        println!("b1 {:?}", b1);
    }
}

// helpers
// Notice, seen a stack of nodes the `next` field has erased type
// We assume a ZST and never print the data field.
impl<T> fmt::Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(data, {})",
            match self.next {
                Some(node) => {
                    let node: &Node<()> = unsafe { transmute(node) };
                    format!("{}", node)
                }
                None => "None".to_string(),
            }
        )
    }
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Stack @{:p}: ({})",
            self,
            match self.head.get() {
                Some(node) => {
                    let node: &Node<()> = unsafe { transmute(node) };
                    format!("{}", node)
                }
                None => "None".to_string(),
            }
        )
    }
}
