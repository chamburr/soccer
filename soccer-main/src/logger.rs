use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_time::Timer;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

pub async fn init(spawner: &Spawner, usb: USB) {
    let driver = Driver::new(usb, Irqs);

    spawner.spawn(logger_task(driver)).unwrap();

    Timer::after_millis(1000).await;
}
