#![no_std]

use bytemuck::{Pod, Zeroable};

pub const FIELD_SZ: usize = size_of::<u32>();
pub const ENTITY_SZ: usize = 31;

pub type RawFields = [u32; ENTITY_SZ];

#[repr(C, packed(4))]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct EngineFields {
    pub position: PipePosition,
    pub velocity: [f32; 2],
    pub target_velocity: [f32; 2],
    pub max_acceleration: f32,
    pub max_speed: f32,
    pub color: [f32; 3],
    pub model: u32,
}

#[repr(C, packed(4))]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct PipePosition {
    pub angle: f32,
    pub depth: f32,
}

#[repr(C, packed(4))]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct Entity {
    pub engine_fields: EngineFields,
    pub game_fields:
        [u32; (size_of::<RawFields>() - size_of::<EngineFields>()) / FIELD_SZ],
}
