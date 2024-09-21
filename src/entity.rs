use crate::visual;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

#[derive(Clone, Copy)]
pub struct PipePosition {
    pub angle: f32,
    pub depth: f32,
}

pub type EntRef = Rc<RefCell<Entity>>;
pub type WeakEntRef = Weak<RefCell<Entity>>;

#[derive(Clone)]
pub struct Entity {
    id: u64,
    pub parent: WeakEntRef,
    pub pos: PipePosition,
    pub color: [f32; 3],
    pub model: usize,
}

impl Entity {
    fn new(id: u64) -> Entity {
        Entity {
            id,
            parent: Weak::default(),
            pos: PipePosition {
                angle: 0f32,
                depth: 0f32,
            },
            color: [1f32; 3],
            model: 0,
        }
    }
}

impl visual::Instance for Entity {
    #[rustfmt::skip]
    fn transform(&self) -> [f32; 12] {
        let (sin, cos) = self.pos.angle.sin_cos();

        [
            sin,  cos,  0f32, cos,
            -cos, sin,  0f32, sin,
            0f32, 0f32, 1f32, self.pos.depth,
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
struct HashEnt(EntRef);

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

    fn borrow_mut(&self) -> RefMut<Entity> {
        self.0.borrow_mut()
    }

    fn unwrap(&self) -> EntRef {
        Rc::clone(&self.0)
    }
}

impl Eq for HashEnt {}

pub struct Manager {
    next_id: u64,
    entities: HashSet<HashEnt>,
}

impl Default for Manager {
    fn default() -> Self {
        Manager {
            next_id: 0u64,
            entities: HashSet::new(),
        }
    }
}

use std::borrow::Borrow;

impl Manager {
    pub fn iter<'a>(
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
