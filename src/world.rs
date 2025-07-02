use crate::FRAME_DURATION_F32;
use crate::{PipePosition, entity, visual};
use entity::EntRef;
use std::cell::RefCell;
use std::rc::Rc;
use visual::WorldPosition;
use visual::geo;

const RING_RADIUS: f32 = 1.07;
const ZOOM_SPEED: f32 = 6.0;

pub struct World {
    rings: Vec<RingInstance>,
    ent_mgr: entity::Manager,
    progress: Rc<RefCell<f32>>,
}

impl World {
    pub fn new(builder: &mut visual::ManagerBuilder, ring_ct: u32) -> Self {
        let vertices = geo::circle_pts(20, RING_RADIUS);
        let indices = geo::loop_indices(20);
        let ring_mesh = (visual::BaseMesh { vertices, indices }).thicken();
        let ring_model = builder.register_model(ring_mesh);
        let progress = Rc::new(RefCell::new(0.0));

        let rings = (0..ring_ct)
            .map(|i| {
                RingInstance::new(
                    visual::WorldPosition([0f32, 0f32, i as f32]),
                    ring_model,
                    Rc::clone(&progress),
                )
            })
            .collect();

        Self {
            rings,
            ent_mgr: Default::default(),
            progress,
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
        let ent = self.ent_mgr.create();
        ent.borrow_mut().position = position;
        ent
    }

    pub fn remove_entity(&mut self, entity: EntRef) {
        self.ent_mgr.remove(&entity);
    }

    pub fn update(&mut self) {
        self.update_logic();
        self.update_physics();
        *self.progress.borrow_mut() += FRAME_DURATION_F32;
    }

    fn update_logic(&mut self) {
        let ents = self.ent_mgr.iter().collect::<Vec<_>>();

        for ent in ents {
            let cloned = Rc::clone(&ent);
            let think = Rc::clone(&ent.borrow().think);
            think(self, cloned);
        }
    }

    fn update_physics(&self) {
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

            ent.position.angle +=
                0.5 * FRAME_DURATION_F32 * FRAME_DURATION_F32 * accel
                    + FRAME_DURATION_F32 * vel_angular;

            ent.position.depth += FRAME_DURATION_F32 * vel_depth;

            if targ_vel_angular > vel_angular {
                vel_angular += FRAME_DURATION_F32 * ent.max_acceleration;
                vel_angular = vel_angular.min(targ_vel_angular);
            } else if targ_vel_angular < vel_angular {
                vel_angular -= FRAME_DURATION_F32 * ent.max_acceleration;
                vel_angular = vel_angular.max(targ_vel_angular);
            }

            ent.velocity = [vel_angular, vel_depth];
        }
    }
}

#[derive(Clone)]
struct RingInstance {
    position: WorldPosition,
    model: usize,
    progress: Rc<RefCell<f32>>,
}

impl RingInstance {
    pub fn new(
        position: WorldPosition,
        model: usize,
        progress: Rc<RefCell<f32>>,
    ) -> Self {
        Self {
            position,
            model,
            progress,
        }
    }
}

impl visual::Instance for RingInstance {
    #[rustfmt::skip]
    fn transform(&self) -> visual::TransformMatrix {
        let WorldPosition([x, y, z]) = self.position;
        let offset = (*self.progress.borrow() * ZOOM_SPEED).rem_euclid(1.0);

        [
            1f32, 0f32, 0f32, x,
            0f32, 1f32, 0f32, y,
            0f32, 0f32, 1f32, z - offset,
        ]
    }

    fn color(&self) -> visual::Color {
        [0.55f32, 0.55f32, 0.55f32]
    }

    fn model(&self) -> usize {
        self.model
    }
}
