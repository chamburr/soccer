use embassy_rp::{peripherals::CORE1, Peripherals};

macro_rules! make_peripherals {
    ($name:ident, ($($pin:ident), *)) => {
        paste::paste! {
            #[allow(non_snake_case)]
            pub struct $name {
                $(pub $pin: embassy_rp::peripherals::$pin,)*
            }

            macro_rules! [<$name:snake>] {
                ($p:ident) => {{
                    use crate::peripherals::*;
                    $name {
                        $($pin: $p.$pin,)*
                    }
                }}
            }
        }
    };
}

make_peripherals! {
    PeripheralsBootloader,
    (FLASH, WATCHDOG, DMA_CH1)
}

make_peripherals! {
    PeripheralsButton,
    (BOOTSEL, PIN_5)
}

make_peripherals! {
    PeripheralsModule,
    (PIN_22)
}

make_peripherals! {
    PeripheralsCamera,
    (UART0, PIN_27, PIN_17, DMA_CH2)
}

make_peripherals! {
    PeripheralsImu,
    (I2C1, PIN_18, PIN_19, PIN_28)
}

make_peripherals! {
    PeripheralsMotor,
    (PIN_6, PIN_7, PIN_8, PIN_9, PIN_10, PIN_11, PIN_14, PIN_15, PWM_CH3, PWM_CH4, PWM_CH5, PWM_CH7)
}

make_peripherals! {
    PeripheralsNetwork,
    (PIN_23, PIN_24, PIN_25, PIN_29, DMA_CH0, PIO0)
}

make_peripherals! {
    PeripheralsTemts,
    (PIN_0, PIN_1, PIN_2, PIN_3, PIN_4)
}

make_peripherals! {
    PeripheralsUart,
    (UART1, PIN_20, PIN_21, DMA_CH3, DMA_CH4)
}

pub struct Peripherals0 {
    pub bootloader: PeripheralsBootloader,
    pub button: PeripheralsButton,
    pub camera: PeripheralsCamera,
    pub imu: PeripheralsImu,
    pub motor: PeripheralsMotor,
    pub network: PeripheralsNetwork,
    pub temts: PeripheralsTemts,
    pub uart: PeripheralsUart,
    pub module:PeripheralsModule,
}

pub struct Peripherals1 {}

pub fn get_peripherals(p: Peripherals) -> (CORE1, Peripherals0, Peripherals1) {
    (
        p.CORE1,
        Peripherals0 {
            bootloader: peripherals_bootloader!(p),
            button: peripherals_button!(p),
            camera: peripherals_camera!(p),
            imu: peripherals_imu!(p),
            motor: peripherals_motor!(p),
            network: peripherals_network!(p),
            temts: peripherals_temts!(p),
            uart: peripherals_uart!(p),
            module: peripherals_module!(p),
        },
        Peripherals1 {},
    )
}
