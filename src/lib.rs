#![no_std]
use core::fmt::Debug;
use core::mem::size_of;
use core::mem::transmute;
use core::mem::MaybeUninit;

mod heap;
mod my_box;
mod stack;

pub use heap::*;
pub use my_box::*;
pub use stack::*;

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

    pub fn init_heap(&self) -> &Alloc {
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

use core::cell::{Cell, UnsafeCell};
pub struct H<const N: usize, const S: usize> {
    heap: MaybeUninit<UnsafeCell<[u8; N]>>,
    start: Cell<usize>,
    end: Cell<usize>,
    stacks: MaybeUninit<[Stack; S]>,
}

impl<const N: usize, const S: usize> H<N, S> {
    const fn new() -> Self {
        Self {
            heap: MaybeUninit::uninit(),
            start: Cell::new(0),
            end: Cell::new(N),
            stacks: MaybeUninit::uninit(),
        }
    }
    fn init(&'static self) -> HI<N, S> {
        let free_stacks = unsafe { &*self.stacks.as_ptr() };

        for stack in free_stacks.iter() {
            stack.head.set(None);
        }

        HI { h: self }
    }
}
unsafe impl<const N: usize, const S: usize> Sync for H<N, S> {}

pub struct HI<const N: usize, const S: usize> {
    h: &'static H<N, S>,
}

impl<const N: usize, const S: usize> HI<N, S> {
    fn start(&self) -> &Cell<usize> {
        &self.h.start
    }
    fn end(&self) -> &Cell<usize> {
        &self.h.end
    }
    fn stacks(&self) -> &[Stack; S] {
        unsafe { self.h.stacks.assume_init_ref() }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::drop;

    #[test]
    fn test_h() {
        type He = H<128, 128>;
        static H: He = H::new();
        let h = H.init();
        for s in h.stacks().iter() {
            assert!(s.pop::<()>() == None);
        }
    }

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
