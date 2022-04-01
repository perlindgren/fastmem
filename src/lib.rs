// #![no_std]
use core::fmt::Debug;

use core::marker::PhantomData;
use core::mem::{align_of, align_of_val, size_of, transmute, MaybeUninit};

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

struct UnInit;
struct Init;

// #[repr(C)]
// pub struct H<const N: usize, const S: usize, State> {
//     heap: MaybeUninit<UnsafeCell<[u8; N]>>,
//     start: Cell<usize>,
//     end: Cell<usize>,
//     stacks: MaybeUninit<[Stack; S]>,
//     _state: PhantomData<State>,
// }

// impl<const N: usize, const S: usize> H<N, S, UnInit> {
//     // creates an uninitialized Heap<_, _, UnInit>
//     // cannot be used unless initialized
//     const fn new() -> Self {
//         Self {
//             heap: MaybeUninit::uninit(),
//             start: Cell::new(0),
//             end: Cell::new(N),
//             stacks: MaybeUninit::uninit(),
//             _state: PhantomData,
//         }
//     }

//     // returns a reference to an initialized Heap<_ ,_ , Init>
//     fn init(&'static self) -> &H<N, S, Init> {
//         let free_stacks = unsafe { &*self.stacks.as_ptr() };

//         for stack in free_stacks.iter() {
//             stack.head.set(None);
//         }

//         unsafe { transmute(self) }
//     }
//     fn init2(&'static self) -> &He<N, S> {
//         let free_stacks = unsafe { &*self.stacks.as_ptr() };

//         for stack in free_stacks.iter() {
//             stack.head.set(None);
//         }

//         unsafe { transmute(self) }
//     }
// }
// unsafe impl<const N: usize, const S: usize, State> Sync for H<N, S, State> {}

#[repr(C)]
pub struct H<const N: usize, const S: usize> {
    heap: MaybeUninit<UnsafeCell<[u8; N]>>,
    start: Cell<usize>,
    end: Cell<usize>,
    stacks: MaybeUninit<[Stack; S]>,
}

impl<const N: usize, const S: usize> H<N, S> {
    // creates an uninitialized Heap<_, _, UnInit>
    // cannot be used unless initialized
    const fn new() -> Self {
        Self {
            heap: MaybeUninit::uninit(),
            start: Cell::new(0),
            end: Cell::new(0),
            stacks: MaybeUninit::uninit(),
        }
    }

    fn init(&'static self) -> &He<N, S> {
        let free_stacks = unsafe { &*self.stacks.as_ptr() };

        for stack in free_stacks.iter() {
            stack.head.set(None);
        }

        let start = self.heap.as_ptr() as usize;
        let end = start + N;

        self.start.set(start);
        self.end.set(end);

        unsafe { transmute(self) }
    }
}
unsafe impl<const N: usize, const S: usize> Sync for H<N, S> {}

#[repr(C)]
pub struct He<const N: usize, const S: usize> {
    heap: UnsafeCell<[u8; N]>,
    start: Cell<usize>,
    end: Cell<usize>,
    stacks: [Stack; S],
}

impl<const N: usize, const S: usize> He<N, S> {
    // Allocate data from heap for T
    //
    // Safety:
    // Alignment of new allocations according to T.
    // After alloc the data must be written first before used.
    //
    // Todo:
    // Drop semantics defined here or elsewhere
    #[inline(always)]
    pub fn alloc<T>(&'static self) -> &mut T {
        let mut start = self.start.get();

        let size = size_of::<T>();
        let align = align_of::<T>();
        let spill = start % align;

        if spill != 0 {
            start += align - spill;
        }

        let new_start = start + size;
        if new_start > self.end.get() {
            panic!("Out of memory (OOM)");
        }

        println!("alloc: size {}, align {}, spill {}", size, align, spill);

        let r: &mut T = unsafe { transmute::<_, &mut T>(start) };
        // let _alloc = core::mem::ManuallyDrop::new(alloc);

        self.start.set(new_start);
        r
    }

    // Allocate a Box like deference to data of size T
    #[inline(always)]
    pub fn box_new<T>(&'static self, t: T) -> Box<T> {
        let index = size_of::<T>();

        let stack = &self.stacks[index];
        let node: &mut Node<T> = match stack.pop() {
            Some(node) => {
                if align_of::<T>() != align_of_val(node) {
                    panic!("illegal alignment");
                };
                node
            }
            None => {
                // new allocation
                println!("new_allocation");
                self.alloc()
            }
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
    fn test_h() {
        static H: H<128, 128> = H::new();
        let h: &'static _ = H.init();

        // Assert that stacks are empty
        for s in h.stacks.iter() {
            assert!(s.pop::<()>() == None);
        }

        // Assert heap start and end
        assert_eq!(h as *const _ as usize, h.start.get());
        assert_eq!(h as *const _ as usize + 128, h.end.get());

        let n_u8 = h.box_new(8u8);
        core::mem::forget(n_u8);
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

    #[test]
    fn test_closure() {
        pub static HEAP: Heap = Heap::new();
        pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
        HEAP.init(&HEAP_DATA);
        pub static ALLOC: AllocTmp = AllocTmp::new(&HEAP);
        let alloc = ALLOC.init();

        let n_closure = alloc.box_new(|x: i32| x + 1);

        assert_eq!(n_closure(1), 2);
        let n_u32 = alloc.box_new(32u32);

        // drop(n_u8); // force drop
        drop(n_u32); // force drop

        let n_u8 = alloc.box_new(8u8);
        let n_u32 = alloc.box_new(32u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop
    }
}

struct A {}
struct B {}

struct C<const Z: usize, X> {
    z: [u32; Z],
    _m: PhantomData<X>,
}

impl<const Z: usize> C<Z, A> {
    const fn new() -> Self {
        Self {
            z: [0u32; Z],
            _m: PhantomData,
        }
    }

    fn to_b(&self) -> &C<Z, B> {
        unsafe { transmute(self) }
    }
}
