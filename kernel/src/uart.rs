use core::fmt::Write;
use core::option::Option::{self, Some, None};
use core::result::Result::Ok;
use lazy_static::lazy_static;

use spin::Mutex;

pub const UART_BASE_ADDR: usize = 0x1000_0000;

// the UART control registers.
// some have different meanings for read vs write.
// see http://byterunner.com/16550.html

const RHR: usize = 0;   // receive holding register (for input bytes)
const THR: usize = 0;   // transmit holding register (for output bytes)
const IER: usize = 1;   // interrupt enable register
const FCR: usize = 2;   // FIFO control register
const LCR: usize = 3;   // line control register
const LSR: usize = 5;                 // line status register
const LSR_TX_IDLE: u8 = 1 << 5;    // THR can accept another character to send

pub struct Uart {
    base_addr: usize,
}

impl Uart {
    pub fn new(base_addr: usize) -> Self {
        Uart { base_addr }
    }

    pub fn get(&self) -> Option<u8> {
        let ptr = self.base_addr as *mut u8;
        if unsafe { ptr.add(LCR).read_volatile() } & 1 == 0 {
            // DR (Data ready) bit set to 0 -> no data
            None
        } else {    // DR bit 1 -> data
            Some(unsafe { ptr.add(RHR).read_volatile() })
        }
    }
    pub fn put(&self, c: u8) {
        let ptr = self.base_addr as *mut u8;
        loop {
            if unsafe { ptr.add(LSR).read_volatile() } & LSR_TX_IDLE != 0 {
                break;
            }
        }
        unsafe {
            ptr.add(THR).write_volatile(c);
        }
    }

    pub fn init(&mut self) {
        let ptr = self.base_addr as *mut u8;
        unsafe {
            // First, set the word length, which
            // are bits 0, and 1 of the line control register (LCR)
            // which is at base_address + 3
            // We can easily write the value 3 here or 0b11, but I'm
            // extending it so that it is clear we're setting two individual
            // fields
            //         Word 0     Word 1
            //         ~~~~~~     ~~~~~~
            let lcr = (1 << 0) | (1 << 1);
            ptr.add(LCR).write_volatile(lcr);

            // Now, enable the FIFO, which is bit index 0 of the FIFO
            // control register (FCR at offset 2).
            // Again, we can just write 1 here, but when we use left shift,
            // it's easier to see that we're trying to write bit index #0.
            ptr.add(FCR).write_volatile(1 << 0);

            // Enable receiver buffer interrupts, which is at bit index
            // 0 of the interrupt enable register (IER at offset 1).
            ptr.add(IER).write_volatile(1 << 0);

            // If we cared about the divisor, the code below would set the divisor
            // from a global clock rate of 22.729 MHz (22,729,000 cycles per second)
            // to a signaling rate of 2400 (BAUD). We usually have much faster signalling
            // rates nowadays, but this demonstrates what the divisor actually does.
            // The formula given in the NS16500A specification for calculating the divisor
            // is:
            // divisor = ceil( (clock_hz) / (baud_sps x 16) )
            // So, we substitute our values and get:
            // divisor = ceil( 22_729_000 / (2400 x 16) )
            // divisor = ceil( 22_729_000 / 38_400 )
            // divisor = ceil( 591.901 ) = 592

            // The divisor register is two bytes (16 bits), so we need to split the value
            // 592 into two bytes. Typically, we would calculate this based on measuring
            // the clock rate, but again, for our purposes [qemu], this doesn't really do
            // anything.
            let divisor: u16 = 592;
            let divisor_least: u8 = (divisor & 0xff) as u8;
            let divisor_most: u8 = (divisor >> 8) as u8;

            // Notice that the divisor register DLL (divisor latch least) and DLM (divisor
            // latch most) have the same base address as the receiver/transmitter and the
            // interrupt enable register. To change what the base address points to, we
            // open the "divisor latch" by writing 1 into the Divisor Latch Access Bit
            // (DLAB), which is bit index 7 of the Line Control Register (LCR) which
            // is at base_address + 3.
            ptr.add(3).write_volatile(lcr | 1 << 7);

            // Now, base addresses 0 and 1 point to DLL and DLM, respectively.
            // Put the lower 8 bits of the divisor into DLL
            ptr.add(0).write_volatile(divisor_least);
            ptr.add(1).write_volatile(divisor_most);

            // Now that we've written the divisor, we never have to touch this again. In
            // hardware, this will divide the global clock (22.729 MHz) into one suitable
            // for 2,400 signals per second. So, to once again get access to the
            // RBR/THR/IER registers, we need to close the DLAB bit by clearing it to 0.
            ptr.add(3).write_volatile(lcr);
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            self.put(c);
        }
        Ok(())
    }
}

// we want a function that returns a unified UART object
lazy_static! {
    pub static ref UART: Mutex<Uart> = Mutex::new(Uart::new(UART_BASE_ADDR));
}
pub unsafe fn init() {
    UART.lock().init();
}