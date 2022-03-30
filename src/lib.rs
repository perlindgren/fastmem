use core::fmt::Debug;
use core::mem::size_of;
use core::mem::transmute;
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
    #[inline(always)]
    pub fn box_new<T>(&'static self, t: T) -> Box<T> {
        let index = size_of::<T>();
        println!("box_new, index {}", index);
        println!("node size {}", size_of::<Node<T>>());
        let stack = &self.free_stacks[index];
        let node: &mut Node<T> = match stack.pop() {
            Some(node) => {
                println!("found node");
                node
            }
            None => {
                println!("new allocation");
                self.heap.alloc()
            }
        };
        node.data = t;
        node.next = unsafe { transmute(stack) };

        Box::new(node)
    }

    // #[inline(always)]
    // pub fn free<T>(&self, my_box: &mut Box<T>) {
    //     let index = size_of::<T>();
    //     println!("box_free, index {}", index);
    //     self.free_stacks[index].push(my_box.node);
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::drop;
    use core::mem::size_of_val;

    #[test]
    fn test_alloc() {
        pub static HEAP: Heap = Heap::new();
        pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
        HEAP.init(&HEAP_DATA);
        pub static ALLOC: AllocTmp = AllocTmp::new(&HEAP);
        let alloc = ALLOC.init();

        let n_u8 = alloc.box_new(8u8);
        println!("n_u8 {}", *n_u8);

        let n_u32 = alloc.box_new(32u32);
        println!("n_u32 {}", *n_u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop

        println!("alloc.free_stacks[1] {}", alloc.free_stacks[1]);

        let n_u8 = alloc.box_new(8u8);
        println!("n_u8 {}", *n_u8);

        let n_u32 = alloc.box_new(32u32);
        println!("n_u32 {}", *n_u32);
    }
}
