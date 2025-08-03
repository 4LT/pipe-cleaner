#![no_std]

use pipe_cleaner_game_lib::{
    EntityRef,
    GAME_FIELDS_SZ,
    PipePosition,
};
use bytemuck::{Zeroable, Pod};

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct MyFields {
    foo: u32,
    bar: f32,
    me: u64,
    _pad: [u32; GAME_FIELDS_SZ - 4],
}

#[unsafe(no_mangle)]
pub extern "C" fn PIPECLEANER_init() {
    let mut entity_ref = EntityRef::<MyFields>::spawn();
    let handle = entity_ref.handle();
    let entity = &mut *entity_ref;
    entity.game_fields.foo = 13;
    entity.game_fields.bar = 12.7;
    entity.game_fields.me = handle;
    entity.engine_fields.position = PipePosition {
        angle: 13.7,
        depth: 3.2,
    };
}
