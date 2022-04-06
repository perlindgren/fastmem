#![no_std]

use core::{
    cell::{Cell, UnsafeCell},
    mem::{align_of, align_of_val, size_of, transmute, MaybeUninit},
};
use cortex_m_semihosting::hprintln;

mod my_box;
mod stack;

pub use my_box::*;
pub use stack::*;

/// Uninitialized representation of backing storage.
///
/// Type parameters:
///
/// const N:usize, the size of the storage in bytes.
/// const S:usize, allocatable sizes [0..S] in bytes.              
#[repr(C)]
pub struct FastMemStore<const N: usize, const S: usize> {
    heap: MaybeUninit<UnsafeCell<[u8; N]>>,
    start: Cell<usize>,
    end: Cell<usize>,
    stacks: MaybeUninit<[Stack; S]>,
}

impl<const N: usize, const S: usize> FastMemStore<N, S> {
    /// Creates the uninitialized storage.
    pub const fn new() -> Self {
        Self {
            heap: MaybeUninit::uninit(),
            start: Cell::new(0),
            end: Cell::new(0),
            stacks: MaybeUninit::uninit(),
        }
    }

    /// Initialize the storage and returns a FastMem reference.
    pub fn init(&'static self) -> &FastMem<N, S> {
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
unsafe impl<const N: usize, const S: usize> Sync for FastMemStore<N, S> {}

/// Initialized representation of the backing storage.
#[repr(C)]
pub struct FastMem<const N: usize, const S: usize> {
    heap: UnsafeCell<[u8; N]>,
    start: Cell<usize>,
    end: Cell<usize>,
    stacks: [Stack; S],
}

unsafe impl<const N: usize, const S: usize> Sync for FastMem<N, S> {}

impl<const N: usize, const S: usize> FastMem<N, S> {
    // Allocate data from heap for T
    //
    // Safety:
    // Alignment of new allocations according to T.
    // After alloc the data must be written first before used.
    //
    // Todo:
    // Drop semantics defined here or elsewhere
    #[inline(always)]
    fn alloc<T>(&'static self) -> Option<&mut T> {
        let mut start = self.start.get();

        let size = size_of::<T>();
        let align = align_of::<T>();
        let spill = start % align;

        if spill != 0 {
            start += align - spill;
        }

        if cfg!(feature = "trace_semihost") {
            hprintln!("alloc: size {}, align {}, spill {}", size, align, spill);
        }

        let new_start = start + size;
        if new_start > self.end.get() {
            None
        } else {
            let r: &mut T = unsafe { transmute::<_, &mut T>(start) };
            self.start.set(new_start);
            Some(r)
        }
    }

    /// Allocate a Box like deference to data of size T
    /// Panics on OOM
    #[inline(always)]
    pub fn new<T>(&'static self, t: T) -> Box<T> {
        match self.try_new(t) {
            None => panic!("Out of memory"),
            Some(b) => b,
        }
    }

    /// Allocate a Box like deference to data of size T
    /// None on OOM, else Some<Box<T>>
    #[inline(always)]
    pub fn try_new<T>(&'static self, t: T) -> Option<Box<T>> {
        let index = size_of::<T>();

        let stack = &self.stacks[index];
        if cfg!(feature = "trace_semihost") {
            hprintln!("box_new index {}", index);
        }
        let node: &mut Node<T> = match stack.pop() {
            Some(node) => {
                if cfg!(feature = "trace_semihost") {
                    hprintln!("box found @{:p}", node);
                };

                // check alignment of data
                if &node.data as *const _ as usize % align_of::<T>() != 0 {
                    panic!("illegal alignment @{:p}", node);
                };

                node
            }
            None => {
                // new allocation
                let node = self.alloc()?;

                if cfg!(feature = "trace_semihost") {
                    hprintln!("box alloc @{:p}", node);
                };
                node
            }
        };
        node.data = t;
        node.next = unsafe { transmute(stack) };

        Some(Box::new(node))
    }
}

#[cfg(test)]
mod test_lib {
    use super::*;
    use core::mem::drop;
    use core::mem::size_of_val;

    #[test]
    fn test_h() {
        static FM_STORE: FastMemStore<128, 128> = FastMemStore::new();
        let fm: &'static _ = FM_STORE.init();

        // Assert that stacks are empty
        for s in fm.stacks.iter() {
            assert!(s.pop::<()>() == None);
        }

        // Assert heap start and end
        assert_eq!(fm as *const _ as usize, fm.start.get());
        assert_eq!(fm as *const _ as usize + 128, fm.end.get());

        let n_u8 = fm.new(8u8);
        assert_eq!(*n_u8, 8);
    }

    #[test]
    fn test_closure() {
        static FM_STORE: FastMemStore<128, 128> = FastMemStore::new();
        let fm: &'static _ = FM_STORE.init();

        let n_closure = fm.new(|x: i32| x + 1);

        assert_eq!(n_closure(1), 2);
        let s = size_of_val(&n_closure.node.data);
        drop(n_closure);

        let stack = fm.stacks[s].pop::<()>();
        assert!(stack.is_some());
    }

    #[test]
    fn test_heap() {
        static FM_STORE: FastMemStore<128, 128> = FastMemStore::new();
        let fm: &'static _ = FM_STORE.init();

        let start = fm.start.get();
        let end = fm.end.get();
        assert_eq!(end - start, 128);

        // 0000 A B B _
        // 0004 C C C C
        // 0008 D D D D
        // 000c D D _ _
        // 0010 E E E E

        // A u8
        let p1: &mut u8 = fm.alloc().unwrap();
        *p1 = 1;
        assert_eq!(*p1, 1);
        assert_eq!(p1 as *const _ as usize, start);

        // B [u2; 1]
        // start with offset 1, no padding, byte array byte aligned
        let p2: &mut [u8; 2] = fm.alloc().unwrap();
        p2[..].copy_from_slice(&[0, 1]);
        assert_eq!(&p2[..], &[0, 1]);
        assert_eq!(p2 as *const _ as usize, start + 1);

        // C u32
        // check padding, with 1
        let p3: &mut u32 = fm.alloc().unwrap();
        *p3 = 0x1234_5678u32;
        assert_eq!(*p3, 0x1234_5678u32);
        assert_eq!(p3 as *const _ as usize, start + 4);

        // D [u16; 3]
        // start with offset 1, no padding, byte array byte aligned
        let p4: &mut [u16; 3] = fm.alloc().unwrap();
        p4[..].copy_from_slice(&[0, 1, 2]);
        assert_eq!(&p4[..], &[0, 1, 2]);
        assert_eq!(p4 as *const _ as usize, start + 8);

        // E u32
        // start with offset 1, no padding, byte array byte aligned
        let p5: &mut u32 = fm.alloc().unwrap();
        *p5 = 0x1234_5678u32;
        assert_eq!(*p5, 0x1234_5678u32);
        assert_eq!(p5 as *const _ as usize, start + 0x10);

        // Double check for overwrites
        assert_eq!(*p1, 1);
        assert_eq!(&p2[..], &[0, 1]);
        assert_eq!(*p3, 0x1234_5678u32);
        assert_eq!(&p4[..], &[0, 1, 2]);
        assert_eq!(*p5, 0x1234_5678u32);
    }

    #[test]
    fn test_align_rust() {
        struct A(u32);
        assert_eq!(size_of::<A>(), 4);

        #[repr(align(8))]
        struct A8(u32);
        assert_eq!(size_of::<A8>(), 8);
        assert_eq!(align_of::<A8>(), 8);
        struct AA8 {
            a: A,   // pad 4, before a8, makes sense
            a8: A8, // pad 4, after a8, makes no sense
        }
        assert_eq!(size_of::<AA8>(), 16); // makes no sense, should be 12
        assert_eq!(align_of::<AA8>(), 8); // makes sense, as padding of 4 between a, and a8

        let aa8 = AA8 { a: A(1), a8: A8(2) };
        assert_eq!(size_of_val(&aa8.a), 4); // ok, size excludes padding
        assert_eq!(size_of_val(&aa8.a8), 8); // ok,

        assert_eq!(align_of_val(&aa8.a), 4); // ok,
        assert_eq!(align_of_val(&aa8.a8), 8); // ok,

        #[repr(align(16))]
        struct A16(u32);
        let a16 = A16(0);
        assert_eq!(size_of::<A16>(), 16);
        assert_eq!(align_of::<A16>(), 16);
        assert_eq!(size_of_val(&a16.0), 4);
        assert_eq!(align_of_val(&a16.0), 4);
    }

    #[test]
    fn test_align() {
        static FM_STORE: FastMemStore<128, 128> = FastMemStore::new();
        let fm: &'static _ = FM_STORE.init();

        let a4 = fm.new([0u8, 1, 2, 4]);

        assert_eq!(align_of_val(&a4[0]), 1);
        // assert_eq!(size_of_val(&a8.node.data), 4);

        // assert_eq!(align_of_val(&*a8), 8);

        // assert_eq!(size_of_val(&*a8), 4);
    }
}
