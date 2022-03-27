use core::cell::Cell;
use core::fmt;
use core::ptr::NonNull;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Node {
    data: usize,
    next: Cell<Option<NonNull<Node>>>,
}

impl Node {
    pub const fn new(data: usize) -> Self {
        Node { data, next: None }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {})",
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Stack {
    pub head: Option<NonNull<Node>>,
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Stack({})",
            match self.head {
                Some(node) => {
                    let node = unsafe { node.as_ref() };
                    format!("{}", node)
                }
                None => "None".to_string(),
            }
        )
    }
}

impl Stack {
    pub const fn new() -> Self {
        Stack { head: None }
    }

    /// push a node to the top of the stack
    pub fn push(&mut self, n: &mut Node) {
        n.next.set(self.head);
        self.head = NonNull::new(n)
    }

    /// pop a node from the top of the stack
    pub fn pop(&mut self) -> Option<&mut Node> {
        match self.head {
            Some(mut node) => {
                let node = unsafe { node.as_mut() };
                self.head = node.next;

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
        let mut stack = Stack::new();

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
        assert_eq!(stack.head, None);
    }
}
