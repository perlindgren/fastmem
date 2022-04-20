use core::{cell::Cell, fmt, mem::transmute, ptr::NonNull};

type Erased = ();

#[derive(Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Node<T> {
    // Non-null pointer to data
    pub ptr: Option<NonNull<T>>,
    // Mutable non-null pointer to next node or head of stack
    pub next: Cell<Option<NonNull<Node<T>>>>,
}

impl<T> Node<T> {
    #[allow(unused)]
    pub(crate) const fn new(ptr: Option<NonNull<T>>, next: Option<NonNull<Node<T>>>) -> Self {
        Node {
            ptr: unsafe { transmute(ptr) },
            next: Cell::new(None),
        }
    }

    #[inline(always)]
    unsafe fn erase(&self) -> &Node<Erased> {
        transmute(self)
    }

    // Safety:
    // Data pointed to by `ptr` is always mutable
    #[inline(always)]
    pub(crate) fn ptr_as_mut_ref(&self) -> &mut T {
        unsafe { transmute(self.ptr) }
    }

    #[inline(always)]
    pub(crate) fn ptr_as_ref(&self) -> &T {
        unsafe { transmute(self.ptr) }
    }
}

impl Node<Erased> {
    // Safety:
    // `Node.ptr` must always point a valid allocation for `T`
    #[inline(always)]
    unsafe fn restore<T>(&self) -> &Node<T> {
        transmute(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub(crate) head: Cell<Option<NonNull<Node<Erased>>>>,
}

impl Stack {
    #[allow(unused)]
    pub(crate) const fn new() -> Self {
        Stack {
            head: Cell::new(None),
        }
    }

    /// push a node to the top of the stack
    #[inline(always)]
    pub(crate) fn push<T>(&self, node: &Node<T>) {
        // erase the concrete type
        let node = unsafe { node.erase() };

        // update the next field to point to head
        node.next.set(self.head.get());

        // update the head to point to the new (erased) Node
        self.head.set(Some(node.into()));
    }

    /// pop a node from the top of the stack
    #[inline(always)]
    pub(crate) fn pop<T>(&self) -> Option<&Node<T>> {
        match self.head.get() {
            Some(node) => {
                // treat the NonNull<Node<Erased>> as &Node<Erased>
                let node = unsafe { node.as_ref() };

                // update head of stack
                self.head.set(node.next.get());

                // restore &Node<Erased> to &Node<T>
                Some(unsafe { node.restore() })
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
        assert_eq!(size_of_val(&l), size_of::<usize>());
        assert_eq!(size_of::<Stack>(), size_of::<usize>());

        let n: Node<Erased> = Node::new(None, None);
        assert_eq!(size_of_val(&n), 2 * size_of::<usize>());
        assert_eq!(size_of::<Node<Erased>>(), 2 * size_of::<usize>());
    }

    #[test]
    fn test_stack_nodes() {
        let stack = Stack::new();

        println!("stack {}", stack);

        let mut n1: Node<Erased> = Node::new(None, None);
        println!("node n1 {}", n1);

        let mut n2: Node<Erased> = Node::new(None, None);
        println!("node n2 {}", n2);

        stack.push(&mut n1);
        println!("stack {}", stack);

        stack.push(&mut n2);
        println!("stack {}", stack);

        let new_n2: &Node<Erased> = stack.pop().unwrap();
        println!("new_n2 {}", new_n2);

        let new_n1: &Node<Erased> = stack.pop().unwrap();
        println!("new_n1 {:?}", new_n1);

        assert_eq!(stack.head.get(), None);
    }

    // use crate::my_box::Box;
    // #[test]
    // fn test_stack_box() {
    //     static mut data: u32 = 42;
    //     static mut N1: Node<u32> = Node::new(NonNull::new(&mut data), None);

    //     println!("n1 {:?}", unsafe { &N1 });
    //     // let b1 = Box::new(unsafe { &mut N1 });
    //     // println!("b1 {:?}", b1);

    //     // Will never be pushed back on stack
    //     // leaking!
    //     // core::mem::forget(b1);
    // }

    // #[test]
    // fn test_alignment() {
    //     #[repr(C, align(32))]
    //     struct A8(u32);

    //     let n = Node::new(A8(8), None);
    //     assert_eq!(align_of_val(&n.data), 8);
    // }
}

impl<T> fmt::Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.next.get() {
            Some(node) => {
                let node = unsafe { node.as_ref().erase() };
                core::fmt::write(f, format_args!("Node({:?}, {})", node.ptr, node))
            }
            None => core::fmt::write(f, format_args!("Node(None)")),
        }
    }
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.head.get() {
            Some(node) => {
                let node: &Node<Erased> = unsafe { node.as_ref() };
                core::fmt::write(f, format_args!("Stack({})", node))
            }
            None => core::fmt::write(f, format_args!("Stack(None)")),
        }
    }
}
