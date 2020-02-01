#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]



//extern crate panic_itm;
extern crate panic_semihosting;

use rtfm::app;
use rtfm::cyccnt::U32Ext;

use stm32h7xx_hal::gpio::{gpiob::PB0, gpiob::PB14, Output, PushPull, GpioExt};
use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::flash::FlashExt;
use stm32h7xx_hal::rcc::RccExt;
use stm32h7xx_hal::pwr::PwrExt;

//use stm32h7xx_hal::i2c::I2cExt;
use stm32h7xx_hal::stm32::I2C1;



use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin};

use cortex_m_semihosting::{ hprintln};


use stm32h7xx_hal::pac::DWT;

use bno080::*;
//use stm32h7xx_hal::stm32::I2C1;

const BLINK_PERIOD: u32 = 10_000_000;



// We need to pass monotonic = rtfm::cyccnt::CYCCNT to use schedule feature of RTFM
#[app(device = stm32h7xx_hal::pac,  peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {
    // Global resources (global variables) are defined here and initialized with the
    // `LateResources` struct in init
    struct Resources {
        user_led1: PB0<Output<PushPull>>,
        user_led3: PB14<Output<PushPull>>,
        i2c1_driver:
        bno080::BNO080<stm32h7xx_hal::i2c::I2c<I2C1, (stm32h7xx_hal::gpio::gpiob::PB8<stm32h7xx_hal::gpio::Alternate<stm32h7xx_hal::gpio::AF4>>, stm32h7xx_hal::gpio::gpiob::PB9<stm32h7xx_hal::gpio::Alternate<stm32h7xx_hal::gpio::AF4>>)>>
        //bno080::BNO080<stm32h7xx_hal::i2c::I2c<stm32h7::stm32h743::I2C1, (stm32h7xx_hal::gpio::gpiob::PB8<stm32h7xx_hal::gpio::Alternate<stm32h7xx_hal::gpio::AF4>>, stm32h7xx_hal::gpio::gpiob::PB9<stm32h7xx_hal::gpio::Alternate<stm32h7xx_hal::gpio::AF4>>)>>

    }

    #[init(schedule = [blinker], spawn=[kicker])]
    fn init(cx: init::Context) -> init::LateResources {
        // Note that interrupts are disabled in `init`
        hprintln!("init").unwrap();

        // Enable cycle counter
        let mut core = cx.core;
        core.DWT.enable_cycle_counter();
        let before = core.DWT.cyccnt.read();
        hprintln!("| {} | before", before).unwrap();

        let device: stm32h7xx_hal::stm32::Peripherals = cx.device;

        // Setup clocks
        let _flash = device.FLASH.constrain();

        // Constrain and Freeze power
        let pwr = device.PWR.constrain();
        let vos = pwr.freeze();

        // Constrain and Freeze clock
        let rcc = device.RCC.constrain();

        //use the existing sysclk
        let mut ccdr = rcc.freeze(vos, &device.SYSCFG);

        // Setup LED
        let gpiob = device.GPIOB.split(&mut ccdr.ahb4);
        let mut led1 = gpiob.pb0.into_push_pull_output();
        led1.set_high().unwrap();
        let mut led3 = gpiob.pb14.into_push_pull_output();
        led3.set_low().unwrap();

        //let gpiob = dp.GPIOB.split(&mut ccdr.ahb4);

        // On NUCLEO-H743ZI2 board, use pins 2 and 4 on CON7
        // (PB8 = I2C_1_SCL, PB9 = I2C_1_SDA)
        let scl = gpiob.pb8.into_alternate_af4().internal_pull_up(true).set_open_drain();
        let sda = gpiob.pb9.into_alternate_af4().internal_pull_up(true).set_open_drain();

        let i2c_dev = device.I2C1.i2c((scl, sda), 400.khz(), &ccdr);
        let i2c1_driver = BNO080::new(i2c_dev);

        // Schedule the blinking task
        cx.schedule.blinker(cx.start + BLINK_PERIOD.cycles()).unwrap();

        cx.spawn.kicker().unwrap();

        hprintln!("| {} | init done", DWT::get_cycle_count() ).unwrap();

        //debug::exit(debug::EXIT_SUCCESS);

        init::LateResources {
            user_led1: led1,
            user_led3: led3,
            i2c1_driver: i2c1_driver,
        }

    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        // interrupts are enabled in `idle`
        hprintln!("| {} | idle start", DWT::get_cycle_count() ).unwrap();
        loop {
            rtfm::export::wfi();
        }
    }

    // Second phase startup: interrupts are enabled
    #[task(resources = [i2c1_driver], spawn = [bar, baz]) ]
    fn kicker(cx: kicker::Context) {
        hprintln!("| {} | kicker start", DWT::get_cycle_count() ).unwrap();

        let res =  cx.resources.i2c1_driver.init();
        if res.is_err() {
            hprintln!("init err {:?}", res).unwrap();
        }

        cx.spawn.bar().unwrap();
        cx.spawn.baz().unwrap();
        hprintln!("| {} | kicker done", DWT::get_cycle_count() ).unwrap();
    }

    #[task]
    fn bar(_cx: bar::Context) {
        hprintln!("| {} | bar done", DWT::get_cycle_count() ).unwrap();
    }

    #[task]
    fn baz(_cx: baz::Context) {
        hprintln!("| {} | baz done",DWT::get_cycle_count() ).unwrap();
    }

    #[task]
    fn handle_i2c1_input(_cx: handle_i2c1_input::Context,  _data: u32) {
        // each command has a four byte header
        hprintln!("| {} | handle_i2c1_input",DWT::get_cycle_count() ).unwrap();
    }

    #[task(resources = [user_led1, user_led3], schedule = [blinker])]
    fn blinker(cx: blinker::Context) {
        // Use the safe local `static mut` of RTFM
        static mut LED_STATE: bool = false;

        //hprintln!("| {} | blinker start", DWT::get_cycle_count() ).unwrap();

        if *LED_STATE {
            cx.resources.user_led1.toggle().unwrap();
            cx.resources.user_led3.toggle().unwrap();
            *LED_STATE = false;
        }
        else {
            cx.resources.user_led1.toggle().unwrap();
            cx.resources.user_led3.toggle().unwrap();
            *LED_STATE = true;
        }
        cx.schedule.blinker(cx.scheduled + BLINK_PERIOD.cycles()).unwrap();

    }

//    fn I2C1_EV();  //event
//    fn I2C1_ER(); //error

    //spawn = [handle_i2c1_input]
    #[task(binds = I2C1_EV, priority = 2, )]
    fn i2c1_ev(_cx: i2c1_ev::Context) {
        // send data to the handler
        hprintln!("| {} | I2C1_EV",DWT::get_cycle_count() ).unwrap();
        //cx.spawn.handle_i2c1_input(42).ok().unwrap();
    }

    // define a list of free/unused interrupts that rtfm may utilize
    // for dispatching software tasks
    extern "C" {
        //fn EXTI0();
        fn EXTI1();
        fn EXTI2();
        fn EXTI3();
    }
};

#[allow(unused)]
fn zippy() {

}
