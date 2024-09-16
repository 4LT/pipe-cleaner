use crate::visual;
use visual::geo;
use visual::WorldPosition;
use std::cell::{Ref, RefCell};

pub struct World {
    ring_model: usize,
    rings: Vec<RingInstance>,
}

impl World {
    pub fn new(builder: &mut visual::ManagerBuilder, ring_ct: u32) -> Self {
        let vertices = geo::circle_pts(20);
        let indices = geo::loop_indices(20);

        let ring_mesh = visual::Mesh {
            vertices,
            indices,
        };

        let ring_model = builder.register_class(ring_mesh);

        let rings = (0..ring_ct).map(|i| {
            RingInstance(WorldPosition([0f32, 0f32, i as f32]), ring_model)
        })
        .collect();

        Self {
            ring_model,
            rings,
        }
    }

    pub fn geometry<'a>(&'a self) -> impl Iterator<Item = &'a (dyn visual::Instance + 'a)> {
        self.rings.iter().map(|r| {
            r as &'a (dyn visual::Instance + 'a)
        })
    }
}

#[derive(Clone, Copy)]
struct RingInstance(WorldPosition, usize);

impl visual::Instance for RingInstance {
    fn transform(&self) -> visual::TransformMatrix {
        let RingInstance(WorldPosition([x, y, z]), _) = self;

        [
            1f32, 0f32, 0f32, *x,
            0f32, 1f32, 0f32, *y,
            0f32, 0f32, 1f32, *z,
        ]
    }

    fn color(&self) -> visual::Color {
        [1f32, 1f32, 1f32]
    }

    fn model(&self) -> usize {
        let RingInstance(_, model) = self;
        *model
    }
}