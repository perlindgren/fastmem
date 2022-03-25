// use core::pin::Pin;
use core::fmt;
use core::mem::size_of;
mod heap;
mod list;

use heap::*;

// #[derive(Debug)]
// pub struct Mem<'a, const B: usize, const S: usize, const N: usize> {
//     mem: [u8; B],
//     free_mem: usize,
//     free_lists: [List<'a>; S],
//     free_nodes: [List<'a>; N],
// }

// impl<'a, const B: usize, const S: usize, const N: usize> Mem<'a, B, S, N> {
//     pub const fn new() -> Mem<'a, B, S, N> {
//         Mem {
//             mem: [0; B],
//             free_mem: 0,
//             free_lists: [List::Null; S],
//             free_nodes: [List::Null; N],
//         }
//     }

//     pub fn alloc<T>(&mut self) -> &T
// where
//        // T: ?Sized,
//     {
//         let size = size_of::<T>();
//         println!("size {}", size);
//         unimplemented!()
//     }

//     pub fn init(&'a mut self) {
//         self.free_mem = B;
//         for i in 0..self.free_nodes.len() - 1 {
//             self.free_nodes[i] = List::Node(&mut self.free_nodes[i + 1] as *const List<'a>, &[]);
//         }
//     }
// }

// another failed attempts
use core::mem::transmute;

// #[derive(Debug)]
// struct M<'a> {
//     m: &'a mut [u8],
// }

// impl<'a> M<'a> {
//     fn alloc<T>(&'a mut self, t: T) -> &mut T {
//         let s = size_of::<T>();
//         println!("size {}", s);
//         let (alloc, free) = self.m.split_at_mut(s);

//         // println!("alloc {:?}, free {:?}", alloc, self);

//         let r: &mut T = unsafe { transmute::<_, &mut T>(alloc.as_mut_ptr()) };
//         let _alloc = core::mem::ManuallyDrop::new(alloc);

//         // core::mem::drop(alloc);
//         *r = t;
//         r
//     }
// }

// #[test]
// fn test_m() {
//     let mut m = M {
//         m: &mut [1, 2, 3, 4],
//     };

//     let p = m.alloc(127u8);

//     println!("p {}", p);

//     let p2 = m.alloc(23u8);
//     //  println!("m {:?}", m);
// }

use core::cell::UnsafeCell;
use core::fmt::Debug;
use core::mem::align_of;
use core::ops::{Deref, DerefMut, Drop};
use std::mem::MaybeUninit;

// struct <'a, const N: usize> Box<T, N>
// where F: Free {

#[derive(Debug)]
struct Box<T> {
    inner: T,
}

impl<T> Box<T> {
    fn new(t: T) -> Self {
        Self { inner: t }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        println!("Dropping HasDrop!");
    }
}

trait Free<T> {
    fn free(&self, v: &T) {}
}

#[derive(Debug)]
struct M<const N: usize> {
    m: UnsafeCell<[u8; N]>,
    free_ptr: UnsafeCell<usize>,
}

impl<const N: usize> M<N> {
    const fn new(m: [u8; N]) -> Self {
        Self {
            m: UnsafeCell::new(m),
            free_ptr: UnsafeCell::new(0),
        }
    }
    fn alloc<T>(&self, t: T) -> Box<&mut T>
    where
        T: Debug,
    {
        let free_ptr = unsafe { &mut *self.free_ptr.get() };
        let m = unsafe { &mut *self.m.get() };

        let size = size_of::<T>();
        let align = align_of::<T>();
        println!("size {}, align {}", size, align);

        let spill = *free_ptr % align;

        if spill != 0 {
            *free_ptr += align - spill;
        }

        println!("free {}", *free_ptr);

        let alloc = &mut m[*free_ptr..*free_ptr + size];

        println!("alloc {:?}", alloc);

        *free_ptr += size;
        // // println!("alloc {:?}, free {:?}", alloc, self);

        let r: &mut T = unsafe { transmute::<_, &mut T>(alloc.as_mut_ptr()) };
        // let _alloc = core::mem::ManuallyDrop::new(alloc);

        // // core::mem::drop(alloc);
        *r = t;
        println!("r {:?}", r);

        Box::new(r)
    }
}

#[test]
fn test_align() {
    let m = M::new([0u8; 16]);

    let p = m.alloc(127u8);
    println!("p {:?}", p);

    let p = m.alloc(127u16);

    println!("p {:?}", p);
    println!("m {:?}", m);

    #[repr(align(8))]
    #[derive(Debug)]
    struct S {
        i: u32,
    }

    let p = m.alloc(S { i: 5 });
    println!("p {:?}", p);

    println!("m {:?}", m);
}

// crazy idea
#[repr(transparent)]
pub struct RacyCell<T>(UnsafeCell<T>);

impl<T> RacyCell<T> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        RacyCell(UnsafeCell::new(value))
    }
    #[inline(always)]
    pub unsafe fn get_mut(&self) -> *mut T {
        self.0.get()
    }

    #[inline(always)]
    pub unsafe fn get(&self) -> *const T {
        self.0.get()
    }
}

unsafe impl<T> Sync for RacyCell<T> {}

use core::cell::Cell;
#[derive(Debug)]
pub struct Heap {
    start: Cell<usize>,
    end: Cell<usize>,
}

unsafe impl Sync for Heap {}

impl Heap {
    const fn new() -> Self {
        Self {
            start: Cell::new(0),
            end: Cell::new(0),
        }
    }

    pub fn init<T>(&self, store: &'static T) {
        let start = store as *const T as usize;
        let end = start + size_of::<T>();
        self.start.set(start);
        self.end.set(end);
    }

    pub fn free_size(&self) -> usize {
        self.end.get() - self.start.get()
    }

    fn alloc<T>(&self, t: T) -> &mut T
    where
        T: Debug,
    {
        let mut start = self.start.get();

        let size = size_of::<T>();
        let align = align_of::<T>();
        let spill = start % align;
        println!(
            "start {:x}, size {}, align {}, spill {}",
            start, size, align, spill
        );

        if spill != 0 {
            start += align - spill;
        }

        println!("start {:x}", start);
        let new_start = start + size;
        if new_start > self.end.get() {
            panic!("oom");
        }

        let r: &mut T = unsafe { transmute::<_, &mut T>(start) };
        // let _alloc = core::mem::ManuallyDrop::new(alloc);

        self.start.set(new_start);
        *r = t;
        println!("r {:?}", r);

        r
    }
}

pub static HEAP: Heap = Heap::new();
pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);

#[test]
fn test_heap() {
    HEAP.init(&HEAP_DATA);
    println!("H {:x?}", HEAP);
    println!("free {}", HEAP.free_size());

    let p = HEAP.alloc(1u8);
    println!("p {}", p);

    let p = HEAP.alloc([0u8, 1]);
    println!("p {:?}", p);

    let p = HEAP.alloc(1u32);
    println!("p {}", p);
}
