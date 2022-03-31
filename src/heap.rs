use core::cell::{Cell, UnsafeCell};
use core::fmt::Debug;
use core::mem::{align_of, size_of, transmute};

#[repr(transparent)]
pub struct RacyCell<T>(UnsafeCell<T>);

impl<T> RacyCell<T> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        RacyCell(UnsafeCell::new(value))
    }
    #[inline(always)]
    pub(crate) unsafe fn get_mut(&self) -> *mut T {
        self.0.get()
    }

    #[inline(always)]
    pub(crate) unsafe fn get(&self) -> *const T {
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
    // Create a new uninitialized heap
    pub const fn new() -> Self {
        Self {
            start: Cell::new(0),
            end: Cell::new(0),
        }
    }

    // Initialize heap from static storage
    pub fn init<T>(&self, store: &'static T) {
        let start = store as *const T as usize;
        let end = start + size_of::<T>();
        self.start.set(start);
        self.end.set(end);
    }

    // Report the free memory in bytes
    pub fn free_size(&self) -> usize {
        self.end.get() - self.start.get()
    }

    // Allocate and initialize T
    #[inline(always)]
    pub fn alloc<T>(&self) -> &mut T {
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

        let r: &mut T = unsafe { transmute::<_, &mut T>(start) };
        // let _alloc = core::mem::ManuallyDrop::new(alloc);

        self.start.set(new_start);
        r
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_heap() {
        use super::*;
        pub static HEAP: Heap = Heap::new();
        pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
        HEAP.init(&HEAP_DATA);
        let start = HEAP.start.get();
        let end = HEAP.end.get();
        assert_eq!(HEAP.free_size(), end - start);

        // 0000 A B B _
        // 0004 C C C C
        // 0008 D D D D
        // 000c D D _ _
        // 0010 E E E E

        // A u8
        let p1: &mut u8 = HEAP.alloc();
        *p1 = 1;
        assert_eq!(*p1, 1);
        assert_eq!(p1 as *const _ as usize, start);

        // B [u2; 1]
        // start with offset 1, no padding, byte array byte aligned
        let p2: &mut [u8; 2] = HEAP.alloc();
        p2[..].copy_from_slice(&[0, 1]);
        assert_eq!(&p2[..], &[0, 1]);
        assert_eq!(p2 as *const _ as usize, start + 1);

        // C u32
        // check padding, with 1
        let p3: &mut u32 = HEAP.alloc();
        *p3 = 0x1234_5678u32;
        assert_eq!(*p3, 0x1234_5678u32);
        assert_eq!(p3 as *const _ as usize, start + 4);

        // D [u16; 3]
        // start with offset 1, no padding, byte array byte aligned
        let p4: &mut [u16; 3] = HEAP.alloc();
        p4[..].copy_from_slice(&[0, 1, 2]);
        assert_eq!(&p4[..], &[0, 1, 2]);
        assert_eq!(p4 as *const _ as usize, start + 8);

        // E u32
        // start with offset 1, no padding, byte array byte aligned
        let p5: &mut u32 = HEAP.alloc();
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
}
