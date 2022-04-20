// #![deny(unsafe_code)]
//#![deny(warnings)]
#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = stm32f4::stm32f411, dispatchers = [EXTI0])]
mod app {
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

        // create a boxed value of type u8 (first allocation)
        let b: Box<u8> = fm.new(1u8);
        hprintln!("{:?}", b);

        drop(b); // explicit drop the box

        // re-allocation of a dropped box
        let b: Box<u8> = fm.new(2u8);
        hprintln!("{:?}", b);

        let l = 2;
        hprintln!("log2 {} {:?}", l, log2(l));

        (Shared {}, Local {}, init::Monotonics())
    }

    pub fn log2(mut v: u32) -> u32 {
        unsafe {
            core::arch::asm!(
                "CTZ {v}, {v}",
                v = inout(reg) v,
            );
        }
        v
    }
}
