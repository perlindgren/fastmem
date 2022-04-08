#![deny(unsafe_code)]
// #![deny(warnings)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = stm32f4::stm32f411, dispatchers = [EXTI0])]
mod app {
    use core::mem::{align_of_val, size_of_val};
    use cortex_m_semihosting::hprintln;
    use fastmem::{Box, FastMem, FastMemStore};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init(local = [
        fm_store: FastMemStore<1024,128> = FastMemStore::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // create an initialized fastmem allocator from the store
        let fm: &'static FastMem<1024, 128> = cx.local.fm_store.init();

        // fastmem is based on the assumption that a Box<T>
        // will refer to a Node<T>, and can be recycled
        // by any other Node<T1>, where size_of::<T> == size_of::<T1>
        //
        // can this assumption be violated in case of alignment requirements
        //
        // Say that T is of size 32, with an alignment of 1 and
        // T1 is of size 32 with an alignment of 32.
        //
        // Can we force an error?
        //
        static FM_STORE: FastMemStore<1024, 128> = FastMemStore::new();
        let fm: &'static _ = FM_STORE.init();

        let n = fm.new([0u8; 44]); // alignment of 1
        assert_eq!(size_of_val(n.node), 48);
        assert_eq!(align_of_val(n.node), 4); // the next (usize) dominates

        // next
        assert_eq!(size_of_val(&n.node.next), 4);
        assert_eq!(align_of_val(&n.node.next), 4);

        // data
        assert_eq!(size_of_val(&n.node.data), 44);
        assert_eq!(align_of_val(&n.node.data), 1);

        drop(n);

        hprintln!("after drop");

        #[repr(C, align(16))]
        struct B {
            data: [u8; 32],
        }

        let n = fm.new(B { data: [0u8; 32] }); // alignment of 1
        assert_eq!(size_of_val(n.node), 48);
        assert_eq!(align_of_val(n.node), 16); // the next (usize) dominates

        // next
        assert_eq!(size_of_val(&n.node.next), 8);
        assert_eq!(align_of_val(&n.node.next), 8);

        // data
        assert_eq!(size_of_val(&n.node.data), 32);
        assert_eq!(align_of_val(&n.node.data), 16);

        let addr = &n.node.data as *const _ as usize;
        assert_eq!(addr % 16, 0);

        (Shared {}, Local {}, init::Monotonics())
    }
}
