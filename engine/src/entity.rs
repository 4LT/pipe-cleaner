use crate::{World, visual};
use std::cell::{Ref, RefCell};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

const PIPE_RADIUS: f32 = 1.0;

#[derive(Clone, Copy)]
pub struct PipePosition {
    pub angle: f32,
    pub depth: f32,
}

pub type Think = dyn Fn(&mut World, EntRef);
pub type EntRef = Rc<RefCell<Entity>>;

pub fn default_think(_: &mut World, _: EntRef) {}

#[derive(Clone)]
pub struct Entity {
    id: u64,
    pub position: PipePosition,
    pub color: [f32; 3],
    pub model: usize,
    pub velocity: [f32; 2],
    pub target_velocity: [f32; 2],
    pub max_acceleration: f32,
    pub max_speed: f32,
    pub countdown: f64,
    pub think: Rc<Think>,
    pub fire: bool,
    pub firing_state: u32,
}

impl Entity {
    fn new(id: u64) -> Entity {
        Entity {
            id,
            position: PipePosition {
                angle: 0f32,
                depth: 0f32,
            },
            color: [1f32; 3],
            model: 0,
            velocity: [0f32; 2],
            target_velocity: [0f32; 2],
            max_acceleration: 0.05,
            max_speed: 1f32,
            countdown: 0f64,
            think: Rc::new(default_think),
            fire: false,
            firing_state: 0,
        }
    }
}

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

#[derive(Clone)]
pub struct HashEnt(EntRef);

impl Hash for HashEnt {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.borrow().id.hash(hasher);
    }
}

impl PartialEq for HashEnt {
    fn eq(&self, other: &HashEnt) -> bool {
        self.borrow().id == other.borrow().id
    }
}

impl HashEnt {
    fn borrow(&self) -> Ref<Entity> {
        (*self.0).borrow()
    }
}

impl Eq for HashEnt {}

pub struct Manager {
    next_id: u64,
    pub entities: HashSet<HashEnt>,
}

impl Default for Manager {
    fn default() -> Self {
        Manager {
            next_id: 0u64,
            entities: HashSet::new(),
        }
    }
}

impl Manager {
    pub fn iter(&self) -> impl Iterator<Item = EntRef> + '_ {
        self.entities.iter().map(|HashEnt(ent)| ent.clone())
    }

    pub fn iter_visual<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a (dyn visual::Instance + 'a)> {
        self.entities
            .iter()
            .map(|HashEnt(ent)| ent as &'a (dyn visual::Instance + 'a))
    }

    pub fn create(&mut self) -> EntRef {
        let ent = Entity::new(self.next_id);
        self.next_id += 1;
        let ent = Rc::new(RefCell::new(ent));
        self.entities.insert(HashEnt(Rc::clone(&ent)));
        return ent;
    }

    pub fn remove(&mut self, ent: &EntRef) {
        self.entities.remove(&HashEnt(Rc::clone(ent)));
    }
}
