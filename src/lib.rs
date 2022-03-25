// use core::pin::Pin;
use core::fmt;
use core::mem::size_of;

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

#[derive(Copy, Clone)]
pub enum List<'a> {
    Node(*const List<'a>, &'a [u8]),
    Null,
}

impl<'a> List<'a> {
    // this works for just push one node to the top
    pub fn push(&'a self, m: &'a [u8]) -> List<'a> {
        List::Node(self, m)
    }

    pub fn pop(&'a self) -> (List<'a>, &'a [u8]) {
        match self {
            List::Node(next, data) => (*List::to_list(*next), data),
            _ => panic!(),
        }
    }

    pub fn to_raw(&self) -> *const List {
        &*self
    }

    pub fn to_list(l: *const List<'a>) -> &List<'a> {
        unsafe { &*(l as *const List<'a>) }
    }
}

impl<'a> fmt::Debug for List<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                List::Node(l, m) => format!("Node ({:?}, {:?})", m, List::to_list(*l)),
                _ => "Null".to_string(),
            }
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::size_of_val;

    #[test]
    fn test_list() {
        let l = List::Null;
        println!("l {:?}", l);

        let l = l.push(&[1]);
        println!("l {:?}", l);

        let l = l.push(&[2]);
        println!("l {:?}", l);

        let (l, d) = l.pop();
        println!("l {:?}, d {:?}", l, d);

        let (l, d) = l.pop();
        println!("l {:?}, d {:?}", l, d);
    }

    // #[test]
    // fn test_m() {
    //     let mut mem: Mem<16, 2, 2> = Mem::new();
    //     mem.init();

    //     println!("m {:?}", mem);
    // }

    #[test]
    fn test() {
        let i = 1;
        let t = move |j: i32| j + i;

        println!("t {}", size_of_val(&t));
    }
}
