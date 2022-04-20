#![cfg_attr(not(test), no_std)]
use core::{
    cell::{Cell, UnsafeCell},
    mem::{align_of, size_of, transmute, MaybeUninit},
    ptr::NonNull,
};
use cortex_m_semihosting::hprintln;

mod my_box;
mod stack;

pub use my_box::*;
pub use stack::*;

#[macro_export]
#[cfg(feature = "trace_semi")]
macro_rules! trace {
    ($($l:tt)*) => {
        hprintln!($($l)*);
    };
}
#[cfg(feature = "trace_std")]
macro_rules! trace {
    ($($l:tt)*) => {
        println!($($l)*);
    };
}

#[cfg(not(any(feature = "trace_std", feature = "trace_semi")))]
macro_rules! trace {
    ($($l:tt)*) => {};
}

pub(crate) use trace;

/// Uninitialized representation of backing storage.
///
/// Type parameters:
///
/// const N:usize, the size of the storage in bytes.
/// const S:usize, allocatable sizes [0..S] in bytes.
///
/// Safety:
/// The heap will be usize aligned (needed later)
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
        // Check that the heap size is usize aligned
        trace!("init");
        assert_eq!(N % size_of::<usize>(), 0);

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
    // Allocate data from heap start for T
    //
    // Safety:
    // - After alloc the data must be written first before used.
    //   (ensured by new below)
    // - OOM results in None
    #[inline(always)]
    fn alloc<T>(&'static self, index: usize) -> Option<&mut T> {
        let aligned_size = 1 << index;

        let start = self.start.get();
        let pad = start % aligned_size;
        let aligned_start = start + aligned_size - pad;
        let aligned_end = aligned_start + aligned_size;

        trace!(
                "alloc: start {:#x}, size<T> {:#x}, index {}, aligned_size {:#x}, pad {:#x}, aligned_start {:x}, aligned_end {:x}",
                start,
                size_of::<T>(),
                index,
                aligned_size,
                pad,
                aligned_start,
                aligned_end,
            );

        if aligned_end > self.end.get() {
            None
        } else {
            let ptr: &mut T = unsafe { transmute(aligned_start) };

            self.start.set(aligned_end);
            Some(ptr)
        }
    }

    // alloc_node
    //
    // Allocate a Node from top of heap
    //
    // Safety:
    // - Node needs to be initialized (ensured on use)
    // - `self.end` is usize aligned
    // - OOM results in None
    #[inline(always)]
    fn alloc_node(&'static self) -> Option<&mut Node> {
        let new_end = self.end.get() - size_of::<Node>();
        trace!(
            "alloc_node: start {:x}, end {:x}, new_end {:x}",
            self.start.get(),
            self.end.get(),
            new_end
        );
        if new_end < self.start.get() {
            None
        } else {
            {
                self.end.set(new_end);
                let node: &mut Node = unsafe { transmute(new_end) };
                Some(node)
            }
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
        let index = 32 - (((size_of::<T>() - 1) as u32).leading_zeros() as usize);
        trace!("try_new, box index {}", index);

        let stack = &self.stacks[index];

        let node: &Node = match stack.pop() {
            Some(node) => {
                trace!("node @{:p}, : {}", node, node);

                // check alignment of data, can be removed later
                if node as *const _ as usize % align_of::<T>() != 0 {
                    panic!("illegal alignment @{:p}", node);
                };

                node
            }
            None => {
                // new allocation
                trace!("new alloc");
                let ptr: &mut T = self.alloc(index)?;
                let node: &mut Node = self.alloc_node()?;

                node.ptr = NonNull::new(unsafe { transmute(ptr) });

                node
            }
        };

        let data_ptr: &mut T = node.ptr_as_mut_ref();
        trace!("data_ptr {:p}", data_ptr);
        trace!("next {:p}", node.next.as_ptr());
        trace!("node {}", node);

        *data_ptr = t;
        node.next.set(unsafe { transmute(stack) });
        trace!("assigned");
        Some(Box::new(node))
    }
}

#[cfg(test)]
mod test_lib {
    use super::*;
    use core::mem::drop;
    use core::mem::{align_of_val, size_of_val};

    // #[test]
    // fn test_alignment() {
    //         static FM_STORE: FastMemStore<1024, 128> = FastMemStore::new();
    //         let fm: &'static _ = FM_STORE.init();

    //         // Assert that stacks are empty
    //         for s in fm.stacks.iter() {
    //             assert!(s.pop::<()>() == None);
    //         }

    //         // Assert heap start and end
    //         assert_eq!(fm as *const _ as usize, fm.start.get());
    //         assert_eq!(fm as *const _ as usize + 1024, fm.end.get());

    //         let n_u8 = fm.new(8u8);
    //         assert_eq!(*n_u8, 8);

    //         // testing on std, usize = 64 (8 bytes)
    //         // the box is a &Node<T>
    //         assert_eq!(size_of_val(&n_u8), 8);
    //         assert_eq!(align_of_val(&n_u8), 8);

    //         // the node on std
    //         // next : Option<NonNull<usize>> (usize)
    //         // data : u8
    //         // next needs to be aligned to 8
    //         // thus rust pads data to next 8, and size becomes 16
    //         assert_eq!(size_of_val(n_u8.node), 16);
    //         assert_eq!(align_of_val(n_u8.node), 8);

    //         // Size and alignment of the next pointer
    //         assert_eq!(size_of_val(&n_u8.node.next), 8);
    //         assert_eq!(align_of_val(&n_u8.node.next), 8);

    //         // Size and alignment of the actual data is 1
    //         assert_eq!(size_of_val(&n_u8.node.data), 1);
    //         assert_eq!(align_of_val(&n_u8.node.data), 1);

    //         // allocate a 9 byte array
    //         let n_9 = fm.new([9u8; 9]);
    //         assert_eq!(*n_9, [9u8; 9]);

    //         // testing on std, usize = 64 (8 bytes)
    //         // the box is a &Node<[9u8; 9]>
    //         assert_eq!(size_of_val(&n_9), 8);
    //         assert_eq!(align_of_val(&n_9), 8);

    //         // the node on std
    //         // next : Option<NonNull<usize>> (usize)
    //         // data : [9u8; 9]
    //         // next needs to be aligned to 8
    //         // thus rust pads data to next 8, and size becomes 24
    //         assert_eq!(size_of_val(n_9.node), 24); // due to the padding
    //         assert_eq!(align_of_val(n_9.node), 8);

    //         // Size and alignment of the next pointer
    //         assert_eq!(size_of_val(&n_9.node.next), 8);
    //         assert_eq!(align_of_val(&n_9.node.next), 8);

    //         // Size and alignment of the actual data is 1
    //         assert_eq!(size_of_val(&n_9.node.data), 9);
    //         assert_eq!(align_of_val(&n_9.node.data), 1);

    //         // Now for something less trivial, small data with large alignment
    //         #[repr(C, align(32))]
    //         #[derive(Debug, PartialEq, Eq)]
    //         enum E {
    //             Dummy,
    //         }

    //         let n_e = fm.new(E::Dummy);
    //         assert_eq!(*n_e, E::Dummy);

    //         // testing on std, usize = 64 (8 bytes)
    //         // the box is a &Node<E>
    //         assert_eq!(size_of_val(&n_e), 8);
    //         assert_eq!(align_of_val(&n_e), 8);

    //         // the node on std
    //         // next : Option<NonNull<usize>> (usize)
    //         // data : E
    //         // next needs to be aligned to 8
    //         // data needs to be aligned to 32
    //         // under repr C,
    //         // largest alignment is 32
    //         // thus rust pads data to next 32, and size becomes 64
    //         assert_eq!(size_of_val(n_e.node), 64); // due to the padding
    //         assert_eq!(align_of_val(n_e.node), 32);

    //         // Size and alignment of the next pointer
    //         assert_eq!(size_of_val(&n_e.node.next), 8);
    //         assert_eq!(align_of_val(&n_e.node.next), 8);

    //         // Size and alignment of the actual data
    //         assert_eq!(size_of_val(&n_e.node.data), 32);
    //         assert_eq!(align_of_val(&n_e.node.data), 32);

    //         // Remark: To my understanding, based on:
    //         // https://doc.rust-lang.org/reference/type-layout.html
    //         //
    //         // With the C repr, the struct will be aligned
    //         // to the largest alignment of any field.
    //         // (in this case 32).
    //         // Size will always be a multiple of the alignment.

    //         // An even more complex example:

    //         #[repr(C, align(16))]
    //         #[derive(Debug, PartialEq, Eq)]
    //         enum E16 {
    //             Dummy,
    //         }

    //         #[repr(C, align(8))]
    //         #[derive(Debug, PartialEq, Eq)]
    //         enum E8 {
    //             Dummy,
    //         }

    //         #[repr(C)]
    //         #[derive(Debug, PartialEq, Eq)]
    //         struct S {
    //             e16: E16, // align 16
    //             e8: E8,   // align 8
    //         }

    //         let n_s = fm.new(S {
    //             e8: E8::Dummy,
    //             e16: E16::Dummy,
    //         });
    //         assert_eq!(
    //             *n_s,
    //             S {
    //                 e8: E8::Dummy,
    //                 e16: E16::Dummy,
    //             }
    //         );

    //         // testing on std, usize = 64 (8 bytes)
    //         // the box is a &Node<E>
    //         assert_eq!(size_of_val(&n_s), 8);
    //         assert_eq!(align_of_val(&n_s), 8);

    //         // the node on std
    //         // next : Option<NonNull<usize>> (usize)
    //         // data :
    //         // S {
    //         //   e8: E8::Dummy,   // align 8
    //         //   e16: E16::Dummy, // align 16
    //         // }
    //         // next needs to be aligned to 8
    //         //
    //         // data itself is a struct that is has a maximum
    //         // alignment requirement of 16.
    //         //
    //         // the size of data is actually 16, padded to 32
    //         // (size needs to be a multiple of alignment).
    //         //
    //         // inside of S
    //         // e8 is size 8 and align 8
    //         // 16 is size 16 and align 16
    //         assert_eq!(size_of_val(n_s.node), 48); // next padded to 16, data 32
    //         assert_eq!(align_of_val(n_s.node), 16); // max overall alignment

    //         // Size and alignment of the next pointer
    //         assert_eq!(size_of_val(&n_s.node.next), 8);
    //         assert_eq!(align_of_val(&n_s.node.next), 8);

    //         // Size and alignment of the actual data
    //         assert_eq!(size_of_val(&n_s.node.data), 32);
    //         assert_eq!(align_of_val(&n_s.node.data), 16);

    //         // Size and alignment of the actual data fields
    //         assert_eq!(size_of_val(&n_s.node.data.e8), 8);
    //         assert_eq!(align_of_val(&n_s.node.data.e8), 8);

    //         assert_eq!(size_of_val(&n_s.node.data.e16), 16);
    //         assert_eq!(align_of_val(&n_s.node.data.e16), 16);
    //     }

    //     #[test]
    //     fn break_alignment() {
    //         // fastmem is based on the assumption that a Box<T>
    //         // will refer to a Node<T>, and can be recycled
    //         // by any other Node<T1>, where size_of::<T> == size_of::<T1>
    //         //
    //         // can this assumption be violated in case of alignment requirements
    //         //
    //         // Say that T is of size 32, with an alignment of 1 and
    //         // T1 is of size 32 with an alignment of 32.
    //         //
    //         // Can we force an error?
    //         //
    //         static FM_STORE: FastMemStore<1024, 128> = FastMemStore::new();
    //         let fm: &'static _ = FM_STORE.init();

    //         let n = fm.new([0u8; 40]); // alignment of 1
    //         assert_eq!(size_of_val(n.node), 48);
    //         assert_eq!(align_of_val(n.node), 8); // the next (usize) dominates

    //         // next
    //         assert_eq!(size_of_val(&n.node.next), 8);
    //         assert_eq!(align_of_val(&n.node.next), 8);

    //         // data
    //         assert_eq!(size_of_val(&n.node.data), 40);
    //         assert_eq!(align_of_val(&n.node.data), 1);

    //         drop(n);

    //         #[repr(C, align(16))]
    //         struct B {
    //             data: [u8; 32],
    //         }

    //         let n = fm.new(B { data: [0u8; 32] }); // alignment of 1
    //         assert_eq!(size_of_val(n.node), 48);
    //         assert_eq!(align_of_val(n.node), 16); // the next (usize) dominates

    //         // next
    //         assert_eq!(size_of_val(&n.node.next), 8);
    //         assert_eq!(align_of_val(&n.node.next), 8);

    //         // data
    //         assert_eq!(size_of_val(&n.node.data), 32);
    //         assert_eq!(align_of_val(&n.node.data), 16);

    //         let addr = &n.node.data as *const _ as usize;
    //         assert_eq!(addr % 16, 0);
    //     }

    //     #[test]
    //     fn test_closure() {
    //         static FM_STORE: FastMemStore<128, 128> = FastMemStore::new();
    //         let fm: &'static _ = FM_STORE.init();

    //         let n_closure = fm.new(|x: i32| x + 1);

    //         assert_eq!(n_closure(1), 2);
    //         let s = size_of_val(&n_closure.node.data);
    //         drop(n_closure);

    //         let stack = fm.stacks[s].pop::<()>();
    //         assert!(stack.is_some());
    //     }

    //     #[test]
    //     fn test_heap() {
    //         static FM_STORE: FastMemStore<128, 128> = FastMemStore::new();
    //         let fm: &'static _ = FM_STORE.init();

    //         let start = fm.start.get();
    //         let end = fm.end.get();
    //         assert_eq!(end - start, 128);

    //         // 0000 A B B _
    //         // 0004 C C C C
    //         // 0008 D D D D
    //         // 000c D D _ _
    //         // 0010 E E E E

    //         // A u8
    //         let p1: &mut u8 = fm.alloc().unwrap();
    //         *p1 = 1;
    //         assert_eq!(*p1, 1);
    //         assert_eq!(p1 as *const _ as usize, start);

    //         // B [u2; 1]
    //         // start with offset 1, no padding, byte array byte aligned
    //         let p2: &mut [u8; 2] = fm.alloc().unwrap();
    //         p2[..].copy_from_slice(&[0, 1]);
    //         assert_eq!(&p2[..], &[0, 1]);
    //         assert_eq!(p2 as *const _ as usize, start + 1);

    //         // C u32
    //         // check padding, with 1
    //         let p3: &mut u32 = fm.alloc().unwrap();
    //         *p3 = 0x1234_5678u32;
    //         assert_eq!(*p3, 0x1234_5678u32);
    //         assert_eq!(p3 as *const _ as usize, start + 4);

    //         // D [u16; 3]
    //         // start with offset 1, no padding, byte array byte aligned
    //         let p4: &mut [u16; 3] = fm.alloc().unwrap();
    //         p4[..].copy_from_slice(&[0, 1, 2]);
    //         assert_eq!(&p4[..], &[0, 1, 2]);
    //         assert_eq!(p4 as *const _ as usize, start + 8);

    //         // E u32
    //         // start with offset 1, no padding, byte array byte aligned
    //         let p5: &mut u32 = fm.alloc().unwrap();
    //         *p5 = 0x1234_5678u32;
    //         assert_eq!(*p5, 0x1234_5678u32);
    //         assert_eq!(p5 as *const _ as usize, start + 0x10);

    //         // Double check for overwrites
    //         assert_eq!(*p1, 1);
    //         assert_eq!(&p2[..], &[0, 1]);
    //         assert_eq!(*p3, 0x1234_5678u32);
    //         assert_eq!(&p4[..], &[0, 1, 2]);
    //         assert_eq!(*p5, 0x1234_5678u32);
    //     }

    //     #[test]
    //     fn test_align_rust() {
    //         struct A(u32);
    //         assert_eq!(size_of::<A>(), 4);

    //         #[repr(align(8))]
    //         struct A8(u32);
    //         assert_eq!(size_of::<A8>(), 8);
    //         assert_eq!(align_of::<A8>(), 8);
    //         struct AA8 {
    //             a: A,   // pad 4, before a8, makes sense
    //             a8: A8, // pad 4, after a8, makes no sense
    //         }
    //         assert_eq!(size_of::<AA8>(), 16); // makes no sense, should be 12
    //         assert_eq!(align_of::<AA8>(), 8); // makes sense, as padding of 4 between a, and a8

    //         let aa8 = AA8 { a: A(1), a8: A8(2) };
    //         assert_eq!(size_of_val(&aa8.a), 4); // ok, size excludes padding
    //         assert_eq!(size_of_val(&aa8.a8), 8); // ok,

    //         assert_eq!(align_of_val(&aa8.a), 4); // ok,
    //         assert_eq!(align_of_val(&aa8.a8), 8); // ok,

    //         #[repr(align(16))]
    //         struct A16(u32);
    //         let a16 = A16(0);
    //         assert_eq!(size_of::<A16>(), 16);
    //         assert_eq!(align_of::<A16>(), 16);
    //         assert_eq!(size_of_val(&a16.0), 4);
    //         assert_eq!(align_of_val(&a16.0), 4);
    //     }

    #[test]
    fn test_align() {
        static FM_STORE: FastMemStore<128, 8> = FastMemStore::new();
        let fm: &'static _ = FM_STORE.init();

        #[repr(C, align(8))]
        struct A8 {
            u8: u8,
        }

        #[repr(C, align(32))]
        struct A32 {
            u8: u8,
        }

        // let a8 = fm.new(A8 { u8: 0 });
        // assert_eq!(size_of_val(&a8.u8), 1);
        // assert_eq!(size_of_val(&*a8), 8);
        // assert_eq!(align_of_val(&*a8), 8);

        let a32 = fm.new(A32 { u8: 0 });
        assert_eq!(size_of_val(&a32.u8), 1);
        assert_eq!(size_of_val(&*a32), 32);
        assert_eq!(align_of_val(&*a32), 32);

        // drop(a);

        // let a1 = fm.new(A { u8: 0 });
        // let a2 = fm.new(A { u8: 0 });
        // let a3 = fm.new(A { u8: 0 });
    }
}
