use core::fmt;

pub static INFO_LOCK: spin::Mutex<()> = spin::Mutex::new(());

#[macro_export]
macro_rules! print
{
	($($args:tt)+) => ({
			use core::fmt::Write;
			let _ = write!(crate::uart::Uart::new(0x1000_0000), $($args)+);
	});
}

#[macro_export]
macro_rules! println
{
	() => ({
		print!("\r\n")
	});
	($fmt:expr) => ({
		print!(concat!($fmt, "\r\n"))
	});
	($fmt:expr, $($args:tt)+) => ({
		print!(concat!($fmt, "\r\n"), $($args)+)
	});
}

#[macro_export]
macro_rules! panic_println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        $crate::print::_panic_print(format_args!($($arg)*));
    })
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
	use core::fmt::Write;
	let mut uart = crate::uart::UART.lock();
	uart.write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _panic_print(args: fmt::Arguments) {
    use core::fmt::Write;
    use crate::uart::*;
	let mut uart = Uart::new(UART_BASE_ADDR);
	uart.write_fmt(args).unwrap();
}

/// Prints an info, with newline.
#[macro_export]
macro_rules! info {
    ($string:expr) => ({
        #[allow(unused_imports)]
        let _info_locker = $crate::print::INFO_LOCK.lock();

        let timestamp = $crate::arch::time();
        let timestamp_subsec_us = timestamp.subsec_micros();

        $crate::print::_print(format_args_nl!(
            concat!("\x1b[0;36m[  {:>3}.{:03}{:03}]\x1b[0m ", $string),
            timestamp.as_secs(),
            timestamp_subsec_us / 1_000,
            timestamp_subsec_us % 1_000
        ));
    });
    ($format_string:expr, $($arg:tt)*) => ({
        #[allow(unused_imports)]
        let _info_locker = $crate::print::INFO_LOCK.lock();

        let timestamp = $crate::arch::time();
        let timestamp_subsec_us = timestamp.subsec_micros();

        $crate::print::_print(format_args_nl!(
            concat!("\x1b[0;36m[  {:>3}.{:03}{:03}]\x1b[0m ", $format_string),
            timestamp.as_secs(),
            timestamp_subsec_us / 1_000,
            timestamp_subsec_us % 1_000,
            $($arg)*
        ));
    })
}
