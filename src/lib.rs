#![no_std]
use core::fmt::Debug;
use core::mem::size_of;
use core::mem::transmute;
use core::mem::MaybeUninit;

mod heap;
mod my_box;
mod stack;

use heap::*;
use my_box::*;
use stack::*;

#[derive(Debug)]
#[repr(C)]
pub struct AllocTmp {
    heap: &'static Heap,
    free_stacks: MaybeUninit<[Stack; 128]>,
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
    #[inline(always)]
    pub fn box_new<T>(&'static self, t: T) -> Box<T> {
        let index = size_of::<T>();

        let stack = &self.free_stacks[index];
        let node: &mut Node<T> = match stack.pop() {
            Some(node) => node,
            None => self.heap.alloc(),
        };
        node.data = t;
        node.next = unsafe { transmute(stack) };

        Box::new(node)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::drop;

    #[test]
    fn test_alloc() {
        pub static HEAP: Heap = Heap::new();
        pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
        HEAP.init(&HEAP_DATA);
        pub static ALLOC: AllocTmp = AllocTmp::new(&HEAP);
        let alloc = ALLOC.init();

        let n_u8 = alloc.box_new(8u8);
        let n_u32 = alloc.box_new(32u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop

        let n_u8 = alloc.box_new(8u8);
        let n_u32 = alloc.box_new(32u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop
    }
}
