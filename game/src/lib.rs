#![no_std]
#![feature(sync_unsafe_cell)]
#![feature(ptr_as_ref_unchecked)]

extern crate alloc;

use core::arch::wasm32::unreachable;
use core::cell::SyncUnsafeCell;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

use bytemuck::{Zeroable, must_cast_mut, must_cast_ref};
use pipe_cleaner_shared as shared;
use shared::{Entity, PipePosition, RawFields};

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

#[global_allocator]
static ALLOCATOR: talc::Talck<spin::Mutex<()>, talc::ErrOnOom> =
    talc::Talc::new(talc::ErrOnOom).lock();

#[unsafe(no_mangle)]
pub extern "C" fn add(left: u8, right: u8) -> u8 {
    left.checked_add(right)
        .ok_or_else(|| panic!("Sum of {left} and {right} overflows"))
        .unwrap()
}

#[unsafe(no_mangle)]
pub extern "C" fn create_and_write_entity() -> u64 {
    let handle = unsafe { PIPECLEANER_create_entity() };

    let mut entity: Entity = Zeroable::zeroed();

    if unsafe { PIPECLEANER_get_entity(handle, &mut entity as _) }
        != 0
    {
        panic!("Failed to acquire entity");
    }

    entity.engine_fields.position = PipePosition {
        angle: 13.7_f32,
        depth: 3.2_f32,
    };

    if unsafe {
        PIPECLEANER_write_entity_back(handle, &entity as _)
    } != 0
    {
        panic!("Failed to write entity back");
    }

    handle
}

#[unsafe(no_mangle)]
pub extern "C" fn read_and_remove_entity(handle: u64) -> f32 {
    let mut entity = Entity::zeroed();

    if unsafe { PIPECLEANER_get_entity(handle, &mut entity as _) } != 0 {
        panic!("Failed to get entity");
    }

    if unsafe { PIPECLEANER_remove_entity(handle) } != 0 {
        panic!("Failed to remove entity");
    }

    let position = entity.engine_fields.position;
    position.angle + position.depth
}

unsafe extern "C" {
    fn PIPECLEANER_create_entity() -> u64;
    fn PIPECLEANER_get_entity(handle: u64, ptr: *mut Entity) -> u32;
    fn PIPECLEANER_write_entity_back(handle: u64, ptr: *const Entity) -> u32;
    fn PIPECLEANER_remove_entity(handle: u64) -> u32;
}
