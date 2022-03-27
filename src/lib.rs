// use core::pin::Pin;
use core::cell::UnsafeCell;
use core::fmt;
use core::fmt::Debug;
use core::mem::align_of;
use core::mem::size_of;
use core::mem::transmute;
use core::ops::{Deref, DerefMut, Drop};
use std::marker::PhantomData;
use std::mem::MaybeUninit;

mod heap;
mod my_box;
mod stack;

use heap::*;
use my_box::*;
use stack::*;

#[derive(Debug)]
#[repr(C)]
pub struct AllocTmp {
    pub heap: &'static Heap,
    pub free_stacks: MaybeUninit<[Stack; 128]>,
}

unsafe impl Sync for AllocTmp {}

impl AllocTmp {
    pub const fn new(heap: &'static Heap) -> Self {
        AllocTmp {
            heap,
            free_stacks: MaybeUninit::uninit(),
        }
    }

    pub fn init(&self) -> &Alloc {
        let free_stacks = unsafe { &*self.free_stacks.as_ptr() };

        for stack in free_stacks.iter() {
            stack.head.set(None);
        }

        unsafe { transmute(self) }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Alloc {
    pub heap: &'static Heap,
    pub free_stacks: [Stack; 128],
}

impl Alloc {
    pub fn box_new<T>(&'static self, t: T) -> Box<&mut T> {
        let index = size_of::<T>();
        println!("box_new, index {}", index);

        match self.free_stacks[index].head.get() {
            Some(node) => {
                panic!("node {:?}", node)
            }
            None => {
                println!("new allocation");
                let n = self.heap.alloc(t);
                let n_addr = n as *const T as usize;
                let n_node = self.heap.alloc(Node::new(n_addr));
                Box::new(n, self, n_node)
            }
        }
    }

    pub fn free<T>(&self, my_box: &mut Box<T>) {
        let index = size_of::<T>();
        println!("box_free, index {}", index);
        self.free_stacks[index].push(my_box.node);
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use core::mem::size_of_val;

//     #[test]
//     fn test_alloc() {
//         pub static HEAP: Heap = Heap::new();
//         pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
//         HEAP.init(&HEAP_DATA);
//         pub static ALLOC: Alloc = Alloc::new(&HEAP);

//         let n = ALLOC.box_new(1);
//     }

//     #[test]
//     fn test_heap_stack() {
//         pub static HEAP: Heap = Heap::new();
//         pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
//         HEAP.init(&HEAP_DATA);

//         let mut s = Stack::new();

//         let n = Node::new(0);
//         println!("n {}", size_of_val(&n));

//         let n1 = HEAP.alloc(Node::new(1));
//         s.push(n1);
//         println!("s {}", s);

//         let n2 = HEAP.alloc(Node::new(2));
//         s.push(n2);
//         println!("s {}", s);

//         let n3 = HEAP.alloc(Node::new(3));
//         s.push(n3);
//         println!("s {}", s);
//     }
// }
