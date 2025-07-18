#![no_std]
#![feature(sync_unsafe_cell)]
#![feature(ptr_as_ref_unchecked)]

use core::arch::wasm32::unreachable;
use core::cell::SyncUnsafeCell;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

const PANIC_MESSAGE_SZ: usize = 256;

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
enum PanicCode {
    NoPanic = 0,
    CompleteReport = 1,
    WriteAborted = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PanicReport {
    code: PanicCode,
    length: u32,
    message: [u8; PANIC_MESSAGE_SZ],
}

impl PanicReport {
    const fn new() -> Self {
        PanicReport {
            code: PanicCode::NoPanic,
            length: 0,
            message: [0; _],
        }
    }
}

struct PanicWriter<'a>(&'a mut PanicReport);

impl<'a> Write for PanicWriter<'a> {
    fn write_str(&mut self, string: &str) -> Result<(), core::fmt::Error> {
        let PanicWriter(report) = self;

        for ch in string.chars() {
            let offset = report.length as usize;
            let byte_len = ch.len_utf8();

            if offset + byte_len > PANIC_MESSAGE_SZ {
                panic!();
            }

            let mut buffer = [0u8; 4];
            ch.encode_utf8(&mut buffer);

            for idx in 0..byte_len {
                report.message[offset + idx] = buffer[idx];
            }

            report.length = (offset + byte_len) as u32;
        }

        Ok(())
    }
}

#[unsafe(no_mangle)]
static PIPECLEANER_panic_report: SyncUnsafeCell<PanicReport> =
    SyncUnsafeCell::new(PanicReport::new());

static IS_PANICKING: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Ok(_) = IS_PANICKING.compare_exchange(
        false,
        true,
        Ordering::SeqCst,
        Ordering::SeqCst,
    ) {
        let report = PIPECLEANER_panic_report.get();

        unsafe { (*report).code = PanicCode::WriteAborted };

        write!(
            PanicWriter(unsafe { report.as_mut_unchecked() }),
            "{}",
            info.message()
        )
        .unwrap();

        unsafe { (*report).code = PanicCode::CompleteReport };
    }

    unreachable();
}

#[unsafe(no_mangle)]
pub extern "C" fn add(left: u8, right: u8) -> u8 {
    left.checked_add(right)
        .ok_or_else(|| panic!("Sum of {left} and {right} overflows"))
        .unwrap()
}
