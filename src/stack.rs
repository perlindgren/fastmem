use core::cell::Cell;
use core::fmt;
use core::ptr::NonNull;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub data: usize,
    next: Option<NonNull<Node>>,
}

impl Node {
    pub const fn new(data: usize) -> Self {
        Node { data, next: None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub head: Cell<Option<NonNull<Node>>>,
}

impl Stack {
    pub const fn new() -> Self {
        Stack {
            head: Cell::new(None),
        }
    }

    /// push a node to the top of the stack
    #[inline(always)]
    pub fn push(&self, n: &mut Node) {
        n.next = self.head.get();
        self.head.set(NonNull::new(n));
    }

    /// pop a node from the top of the stack
    #[inline(always)]
    pub fn pop(&self) -> Option<&mut Node> {
        match self.head.get() {
            Some(mut node) => {
                let node = unsafe { node.as_mut() };
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
    fn test_list() {
        let stack = Stack::new();

        let mut n1 = Node::new(1);
        stack.push(&mut n1);

        let mut n2 = Node::new(2);
        stack.push(&mut n2);

        let n = stack.pop().unwrap();
        assert_eq!(n.data, 2);
        assert_eq!(n.next, None);

        let n = stack.pop().unwrap();
        assert_eq!(n.data, 1);
        assert_eq!(n.next, None);

        let n = stack.pop();
        assert_eq!(n, None);
        assert_eq!(stack.head.get(), None);
    }
}

// helpers

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({:x}, {})",
            self.data,
            match self.next {
                Some(node) => {
                    let node = unsafe { node.as_ref() };
                    format!("{:?}", node)
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
            "Stack({})",
            match self.head.get() {
                Some(node) => {
                    let node = unsafe { node.as_ref() };
                    format!("{}", node)
                }
                None => "None".to_string(),
            }
        )
    }
}