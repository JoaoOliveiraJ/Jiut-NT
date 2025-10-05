use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial = unsafe { SerialPort::new(0x3F8) };
        serial.init();
        Mutex::new(serial)
    };
}

#[doc(hidden)]
pub fn _serial_print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).ok();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_serial_print(core::format_args!($($arg)*));
    }
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\r\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\r\n", core::format_args!($($arg)*)));
}

