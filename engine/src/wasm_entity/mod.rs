mod allocator;

pub use allocator::{Allocator, FIELD_SZ, RawFields};
use bytemuck::{Pod, Zeroable};
use std::num::{NonZero, NonZeroU32};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle {
    bits: u64,
}

impl Handle {
    pub fn from_bits(bits: u64) -> Option<Self> {
        let (id, idx) = (Self::bits_to_id(bits), Self::bits_to_index(bits));

        let id = if let Some(id) = NonZero::new(id) {
            id
        } else {
            return None;
        };

        let idx = if let Some(idx) = NonZero::new(idx) {
            idx
        } else {
            return None;
        };

        Some(Self::new(id, idx))
    }

    pub fn new(id: NonZeroU32, index: NonZeroU32) -> Self {
        Self {
            bits: u64::from(u32::from(id))
                | (u64::from(u32::from(index)) << 32),
        }
    }

    pub fn bits(&self) -> u64 {
        self.bits
    }

    pub fn id(&self) -> u32 {
        Self::bits_to_id(self.bits)
    }

    pub fn index(&self) -> u32 {
        Self::bits_to_index(self.bits)
    }

    fn bits_to_index(bits: u64) -> u32 {
        (bits >> 32) as u32
    }

    fn bits_to_id(bits: u64) -> u32 {
        bits as u32
    }
}

#[repr(C, align(4))]
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

const PIPE_RADIUS: f32 = 1.0;

#[repr(C, align(4))]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct PipePosition {
    pub angle: f32,
    pub depth: f32,
}

#[repr(C, align(4))]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct Entity {
    engine_fields: EngineFields,
    game_fields:
        [u32; (size_of::<RawFields>() - size_of::<EngineFields>()) / FIELD_SZ],
}

/*
impl visual::Instance for Entity {
    #[rustfmt::skip]
    fn transform(&self) -> [f32; 12] {
        let (sin, cos) = self.position.angle.sin_cos();

        [
            cos,  sin,  0f32, PIPE_RADIUS*cos,
            sin, -cos,  0f32, PIPE_RADIUS*sin,
            0f32, 0f32, 1f32, self.position.depth,
        ]
    }

    fn color(&self) -> [f32; 3] {
        self.color
    }

    fn model(&self) -> usize {
        self.model
    }
}
*/
