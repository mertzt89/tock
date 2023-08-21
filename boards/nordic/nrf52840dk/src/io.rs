// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use kernel::utilities::cells::VolatileCell;

use core::fmt::Write;
use core::panic::PanicInfo;
use kernel::hil::uart::Transmit;
use kernel::ErrorCode;

use cortexm4;
use kernel::debug;
use kernel::debug::IoWrite;
use kernel::hil::led;
use kernel::hil::led::Led;
use kernel::hil::uart;
use kernel::hil::uart::Configure;
use kernel::hil::uart::Receive;
use nrf52840::gpio::Pin;

use crate::CHIP;
use crate::PROCESSES;
use crate::PROCESS_PRINTER;

// enum Writer {
//     WriterUart(/* initialized */ bool),
//     WriterRtt(&'static capsules_extra::segger_rtt::SeggerRttMemory<'static>),
// }

// static mut WRITER: Writer = Writer::WriterUart(false);

// fn wait() {
//     for _ in 0..100 {
//         cortexm4::support::nop();
//     }
// }

// /// Set the RTT memory buffer used to output panic messages.
// pub unsafe fn set_rtt_memory(
//     rtt_memory: &'static capsules_extra::segger_rtt::SeggerRttMemory<'static>,
// ) {
//     WRITER = Writer::WriterRtt(rtt_memory);
// }

// impl Write for Writer {
//     fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
//         self.write(s.as_bytes());
//         Ok(())
//     }
// }

struct Writer {
    writes: usize,
}

static mut WRITER: Writer = Writer { writes: 0 };

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        self.write(s.as_bytes());
        Ok(())
    }
}

const BUF_LEN: usize = 512;
static mut STATIC_PANIC_BUF: [u8; BUF_LEN] = [0; BUF_LEN];

static mut DUMMY: DummyUsbClient = DummyUsbClient {
    fired: VolatileCell::new(false),
};

struct DummyUsbClient {
    fired: VolatileCell<bool>,
}

impl uart::TransmitClient for DummyUsbClient {
    fn transmitted_buffer(&self, _: &'static mut [u8], _: usize, _: Result<(), ErrorCode>) {
        // led.on();
        self.fired.set(true);
    }
}

impl uart::ReceiveClient for DummyUsbClient {
    fn received_buffer(
        &self,
        _rx_buffer: &'static mut [u8],
        _rx_len: usize,
        _rval: Result<(), ErrorCode>,
        _error: uart::Error,
    ) {
    }
}

impl IoWrite for Writer {
    fn write(&mut self, buf: &[u8]) -> usize {
        let led2_pin = &nrf52840::gpio::GPIOPin::new(Pin::P0_14);
        let led2 = &mut led::LedLow::new(led2_pin);
        let led_kernel_pin = &nrf52840::gpio::GPIOPin::new(Pin::P0_13);
        let led = &mut led::LedLow::new(led_kernel_pin);
        // led.init();
        // led.off();

        // Here we mimic a synchronous UART output by calling transmit_buffer
        // on the CDC stack and then spinning on USB interrupts until the transaction
        // is complete. If the USB or CDC stack panicked, this may fail. It will also
        // fail if the panic occurred prior to the USB connection being initialized.
        // In the latter case, the LEDs should still blink in the panic pattern.

        // spin so that if any USB DMA is ongoing it will finish
        // we should only need this on the first call to write()
        // if self.writes == 0 {
        //     let mut i = 0;
        //     loop {
        //         i += 1;
        //         cortexm4::support::nop();
        //         if i > 1000 {
        //             break;
        //         }
        //     }
        // }

        // self.writes += 1;

        // copy_from_slice() requires equal length slices
        // This will truncate any writes longer than BUF_LEN, but simplifies the
        // code. In practice, BUF_LEN=512 always seems sufficient for the size of
        // individual calls to write made by the panic handler.
        let mut max = BUF_LEN;
        if buf.len() < BUF_LEN {
            max = buf.len();
        }

        unsafe {
            // If CDC_REF_FOR_PANIC is not yet set we panicked very early,
            // and not much we can do. Don't want to double fault,
            // so just return.
            super::CDC_REF_FOR_PANIC.map(|cdc| {
                // super::NRF52_RTC.map(|rtc| {
                // Lots of unsafe dereferencing of global static mut objects here.
                // However, this should be okay, because it all happens within
                // a single thread, and:
                // - This is the only place the global CDC_REF_FOR_PANIC is used, the logic is the same
                //   as applies for the global CHIP variable used in the panic handler.
                // - We do create multiple mutable references to the STATIC_PANIC_BUF, but we never
                //   access the STATIC_PANIC_BUF after a slice of it is passed to transmit_buffer
                //   until the slice has been returned in the uart callback.
                // - Similarly, only this function uses the global DUMMY variable, and we do not
                //   mutate it.
                let usb = &mut cdc.controller();
                STATIC_PANIC_BUF[..max].copy_from_slice(&buf[..max]);
                let static_buf = &mut STATIC_PANIC_BUF;
                cdc.set_transmit_client(&DUMMY);
                cdc.set_receive_client(&DUMMY);
                match cdc.transmit_buffer(static_buf, max) {
                    Ok(()) => {
                        led.on();
                    }
                    _ => {
                        led2.on();
                    }
                }

                // let mut i = 0;
                // loop {
                //     i += 1;
                //     cortexm4::support::nop();
                //     if i > 10000 {
                //         break;
                //     }
                // }

                let mut interrupt_count = 0;
                loop {
                    // if DUMMY.fired.get() == true {
                    //     led.on();
                    //     // break;
                    // }

                    if let Some(interrupt) = cortexm4::nvic::next_pending() {
                        if interrupt == 39 {
                            led.off();
                            interrupt_count += 1;
                            if interrupt_count >= 2 {}
                            // led.on();

                            let n = cortexm4::nvic::Nvic::new(interrupt);

                            // n.disable();
                            n.clear_pending();
                            n.enable();

                            usb.handle_interrupt();

                            // let mut i = 0;
                            // loop {
                            //     i += 1;
                            //     cortexm4::support::nop();
                            //     if i > 10000 {
                            //         break;
                            //     }
                            // }

                            // } else if interrupt < 25 {
                        } else {
                            // led.off();
                            if interrupt == 6 {
                                // led.off();
                            }
                            // rtc.handle_interrupt();

                            let n = cortexm4::nvic::Nvic::new(interrupt);
                            n.clear_pending();
                            n.enable();
                            // n.clear_pending();
                        }
                    } else {
                        cortexm4::support::wfi();
                    }

                    if DUMMY.fired.get() == true {
                        // buffer finished transmitting, return so we can output additional
                        // messages when requested by the panic handler.
                        // led.off();
                        break;
                    }
                }
                DUMMY.fired.set(false);
                // });
            });
        }
        buf.len()
    }
}

// impl IoWrite for Writer {
//     fn write(&mut self, buf: &[u8]) -> usize {
//         match self {
//             Writer::WriterUart(ref mut initialized) => {
//                 // Here, we create a second instance of the Uarte struct.
//                 // This is okay because we only call this during a panic, and
//                 // we will never actually process the interrupts
//                 let uart = nrf52840::uart::Uarte::new();
//                 if !*initialized {
//                     *initialized = true;
//                     let _ = uart.configure(uart::Parameters {
//                         baud_rate: 115200,
//                         stop_bits: uart::StopBits::One,
//                         parity: uart::Parity::None,
//                         hw_flow_control: false,
//                         width: uart::Width::Eight,
//                     });
//                 }
//                 for &c in buf {
//                     unsafe {
//                         uart.send_byte(c);
//                     }
//                     while !uart.tx_ready() {}
//                 }
//             }
//             Writer::WriterRtt(rtt_memory) => {
//                 let up_buffer = unsafe { &*rtt_memory.get_up_buffer_ptr() };
//                 let buffer_len = up_buffer.length.get();
//                 let buffer = unsafe {
//                     core::slice::from_raw_parts_mut(
//                         up_buffer.buffer.get() as *mut u8,
//                         buffer_len as usize,
//                     )
//                 };

//                 let mut write_position = up_buffer.write_position.get();

//                 for &c in buf {
//                     buffer[write_position as usize] = c;
//                     write_position = (write_position + 1) % buffer_len;
//                     up_buffer.write_position.set(write_position);
//                     wait();
//                 }
//             }
//         };
//         buf.len()
//     }
// }

#[cfg(not(test))]
#[no_mangle]
#[panic_handler]
/// Panic handler
pub unsafe extern "C" fn panic_fmt(pi: &PanicInfo) -> ! {
    cortexm4::nvic::disable_all();
    cortexm4::nvic::clear_all_pending();

    let n = cortexm4::nvic::Nvic::new(39);
    n.enable();

    // The nRF52840DK LEDs (see back of board)
    let led_kernel_pin = &nrf52840::gpio::GPIOPin::new(Pin::P0_13);
    let led = &mut led::LedLow::new(led_kernel_pin);
    let writer = &mut WRITER;

    // let led_kernel_pin = &nrf52840::gpio::GPIOPin::new(Pin::P0_13);
    //     let led = &mut led::LedLow::new(led_kernel_pin);
    led.init();
    led.off();

    let led2pin = &nrf52840::gpio::GPIOPin::new(Pin::P0_14);
    let led2 = &mut led::LedLow::new(led2pin);
    led2.init();
    led2.off();

    // debug::panic_banner(writer, pi);
    // loop {}

    debug::panic(
        &mut [led],
        writer,
        pi,
        &cortexm4::support::nop,
        &PROCESSES,
        &CHIP,
        &PROCESS_PRINTER,
    )
}
