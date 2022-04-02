#![deny(unsafe_code)]
// #![deny(warnings)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = lm3s6965, peripherals = true)]
mod app {

    use core::mem::MaybeUninit;
    use cortex_m_semihosting::{debug, hprintln};
    use fastmem::{He, H};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init(local = [
        heap: H<128, 128> = H::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        hprintln!("init");

        let heap = cx.local.heap.init();

        let n_u8 = heap.box_new(8u8);
        let n_u32 = heap.box_new(32u32);

        hprintln!("{}", *n_u8);
        hprintln!("{}", *n_u32);

        hprintln!("n_u8 @{:p}", &*n_u8);
        hprintln!("n_u32 @{:p}", &*n_u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop

        hprintln!("here");

        let n_u8 = heap.box_new(8u8);
        let n_u32 = heap.box_new(32u32);

        hprintln!("n_u8 @{:p}", &*n_u8);
        hprintln!("n_u32 @{:p}", &*n_u32);

        drop(n_u8); // force drop
        drop(n_u32); // force drop

        debug::exit(debug::EXIT_SUCCESS); // Exit QEMU simulator

        (Shared {}, Local {}, init::Monotonics())
    }
}
