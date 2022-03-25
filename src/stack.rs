use core::fmt;
use core::ptr::NonNull;

#[derive(Copy, Clone, Debug)]
pub struct Node {
    data: usize,
    next: Option<NonNull<Node>>,
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
                    format!("{}", node)
                }
                None => "None".to_string(),
            }
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Stack {
    head: Option<NonNull<Node>>,
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
        n.next = self.head;
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
        let mut l = Stack::new();
        println!("l {}", l);
        let mut n = Node {
            data: 0,
            next: None,
        };

        l.push(&mut n);
        println!("l {}", l);
        println!("n {}", n);

        let mut n2 = Node {
            data: 1,
            next: None,
        };

        l.push(&mut n2);

        println!("l {}", l);

        let n = l.pop().unwrap();
        println!("n {}", n);
        println!("l {}", l);

        let n = l.pop().unwrap();
        println!("n {}", n);
        println!("l {}", l);

        let n = l.pop();
        println!("n {:?}", n);
        println!("l {}", l);
    }

    // #[test]
    // fn test_m() {
    //     let mut mem: Mem<16, 2, 2> = Mem::new();
    //     mem.init();

    //     println!("m {:?}", mem);
    // }

    #[test]
    fn test() {
        let i = 1;
        let t = move |j: i32| j + i;

        println!("t {}", size_of_val(&t));
    }
}
