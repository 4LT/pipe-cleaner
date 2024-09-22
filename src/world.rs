use crate::{visual, entity, PipePosition};
use std::cell::{Ref, RefCell};
use visual::geo;
use visual::WorldPosition;
use crate::FRAME_DURATION;

const FRAME_DURATION_F32: f32 = FRAME_DURATION as f32;
const RING_RADIUS: f32 = 0.57;

pub struct World {
    ring_model: usize,
    rings: Vec<RingInstance>,
    ent_mgr: entity::Manager,
}

impl World {
    pub fn new(builder: &mut visual::ManagerBuilder, ring_ct: u32) -> Self {
        let vertices = geo::circle_pts(20, RING_RADIUS);
        let indices = geo::loop_indices(20);

        let ring_mesh = visual::Mesh { vertices, indices };

        let ring_model = builder.register_class(ring_mesh);

        let rings = (0..ring_ct)
            .map(|i| {
                RingInstance(WorldPosition([0f32, 0f32, i as f32]), ring_model)
            })
            .collect();

        Self {
            ring_model,
            rings,
            ent_mgr: Default::default(),
        }
    }

    pub fn geometry<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a (dyn visual::Instance + 'a)> {
        self.rings
            .iter()
            .map(|r| r as &'a (dyn visual::Instance + 'a))
            .chain(self.ent_mgr.iter_visual())
    }

    pub fn place_entity(&mut self, position: PipePosition) -> entity::EntRef {
        let mut ent = self.ent_mgr.create();
        ent.borrow_mut().position = position;
        ent
    }

    pub fn update_physics(&self) {
        for ent in self.ent_mgr.iter() {
            let mut ent = ent.borrow_mut();
            
            let [mut vel_angular, vel_depth] = ent.velocity;
            let [targ_vel_angular, _] = ent.target_velocity;

            let accel = if targ_vel_angular > vel_angular {
                ent.max_acceleration
            } else if targ_vel_angular < vel_angular {
                -ent.max_acceleration
            } else {
                0.0
            };

            ent.position.angle+= 
                0.5 * FRAME_DURATION_F32 * FRAME_DURATION_F32 * accel
                + FRAME_DURATION_F32 * vel_angular;

            if targ_vel_angular > vel_angular {
                vel_angular+= FRAME_DURATION_F32 * ent.max_acceleration;
                vel_angular = vel_angular.min(targ_vel_angular);
            } else if targ_vel_angular < vel_angular {
                vel_angular-= FRAME_DURATION_F32 * ent.max_acceleration;
                vel_angular = vel_angular.max(targ_vel_angular);
            }

            ent.velocity = [vel_angular, vel_depth];
        }
    }
}

#[derive(Clone, Copy)]
struct RingInstance(WorldPosition, usize);

impl visual::Instance for RingInstance {
    fn transform(&self) -> visual::TransformMatrix {
        let RingInstance(WorldPosition([x, y, z]), _) = self;

        [
            1f32, 0f32, 0f32, *x, 0f32, 1f32, 0f32, *y, 0f32, 0f32, 1f32, *z,
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
