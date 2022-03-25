use core::cell::UnsafeCell;

use core::fmt::Debug;

use core::cell::Cell;
use core::mem::{align_of, size_of, transmute};

// use core::ops::{Deref, DerefMut, Drop};
// use std::mem::MaybeUninit;

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

#[derive(Debug)]
pub struct Heap {
    start: Cell<usize>,
    end: Cell<usize>,
}

unsafe impl Sync for Heap {}

impl Heap {
    pub const fn new() -> Self {
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
