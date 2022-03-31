#![deny(unsafe_code)]
// #![deny(warnings)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = lm3s6965, peripherals = true)]
mod app {

    use cortex_m_semihosting::{debug, hprintln};
    use fastmem::{Alloc, AllocTmp, Heap, RacyCell};
    use core::mem::MaybeUninit;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init(local = [
        HEAP:Heap = Heap::new(), 
        HEAP_DATA:RacyCell<[u8; 128]> = RacyCell::new([0; 128]),
        ALLOC: MaybeUninit<AllocTmp> = MaybeUninit::uninit(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        hprintln!("init");

        // pub static HEAP: Heap = Heap::new();
        // pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
        cx.local.HEAP.init(cx.local.HEAP_DATA);
        let 
        // pub static ALLOC: AllocTmp = AllocTmp::new(&HEAP);
        // let alloc = ALLOC.init();

        // let n_u8 = alloc.box_new(8u8);
        // let n_u32 = alloc.box_new(32u32);

        // drop(n_u8); // force drop
        // drop(n_u32); // force drop

        // let n_u8 = alloc.box_new(8u8);
        // let n_u32 = alloc.box_new(32u32);

        // drop(n_u8); // force drop
        // drop(n_u32); // force drop

        debug::exit(debug::EXIT_SUCCESS); // Exit QEMU simulator

        (Shared {}, Local {}, init::Monotonics())
    }
}
