use super::{Entity, Handle};
use bytemuck::{Pod, Zeroable, must_cast, must_cast_mut, must_cast_ref};
use std::num::{NonZero, NonZeroU32};

use pipe_cleaner_shared::{ENTITY_SZ, FIELD_SZ};

pub const BLOCK_SZ: usize = ENTITY_SZ + 1;

#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
struct BlockMetadata {
    pub id: u32,
}

impl BlockMetadata {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
struct FreeMetadata {
    _id: u32,
    pub next: Option<NonZeroU32>,
}

impl FreeMetadata {
    pub fn new(next: Option<NonZeroU32>) -> Self {
        Self {
            _id: 0u32, // MUST be 0
            next,
        }
    }
}

pub type RawFields = [u32; BLOCK_SZ - size_of::<BlockMetadata>() / FIELD_SZ];

#[repr(C, align(4))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct UnknownBlock {
    metadata: BlockMetadata,
    _rest: [u32; BLOCK_SZ - size_of::<BlockMetadata>() / FIELD_SZ],
}

impl UnknownBlock {
    pub fn new(id: u32) -> Self {
        Self {
            metadata: BlockMetadata::new(id),
            _rest: Default::default(),
        }
    }
}

#[repr(C, align(4))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct OccupiedBlock {
    metadata: BlockMetadata,
    rest: RawFields,
}

impl OccupiedBlock {
    pub fn new(id: NonZeroU32) -> Self {
        Self {
            metadata: BlockMetadata::new(id.into()),
            rest: Default::default(),
        }
    }

    pub fn entity(&self) -> &Entity {
        must_cast_ref(&self.rest)
    }

    pub fn entity_mut(&mut self) -> &mut Entity {
        must_cast_mut(&mut self.rest)
    }
}

#[repr(C, align(4))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FreeBlock {
    metadata: FreeMetadata,
    _rest: [u32; BLOCK_SZ - size_of::<FreeMetadata>() / FIELD_SZ],
}

impl FreeBlock {
    pub fn new(next: Option<NonZeroU32>) -> Self {
        Self {
            metadata: FreeMetadata::new(next),
            _rest: Default::default(),
        }
    }
}

pub struct Allocator {
    memory: Vec<UnknownBlock>,
    free_head: Option<NonZeroU32>,
    next_id: NonZeroU32,
}

impl Allocator {
    pub fn entity(&self, handle: Handle) -> Option<&Entity> {
        self.get_occupied_block(handle).map(|block| block.entity())
    }

    pub fn entity_mut(&mut self, handle: Handle) -> Option<&mut Entity> {
        self.get_occupied_block_mut(handle)
            .map(|block| block.entity_mut())
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.memory[1..].iter().filter_map(|block| {
            if block.metadata.id > 0 {
                Some(must_cast_ref::<_, OccupiedBlock>(block).entity())
            } else {
                None
            }
        })
    }

    pub fn entities_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.memory[1..].iter_mut().filter_map(|block| {
            if block.metadata.id > 0 {
                Some(must_cast_mut::<_, OccupiedBlock>(block).entity_mut())
            } else {
                None
            }
        })
    }

    pub fn alloc(&mut self) -> Handle {
        let idx = self.pop_free();
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).unwrap();

        let block = must_cast(OccupiedBlock::new(id));

        if let Some(idx) = idx {
            self.memory[u32::from(idx) as usize] = block;
            Handle::new(id, idx)
        } else {
            let idx =
                NonZeroU32::new(u32::try_from(self.memory.len()).unwrap())
                    .unwrap();

            self.memory.push(block);
            Handle::new(id, idx)
        }
    }

    pub fn free(&mut self, handle: Handle) -> bool {
        if self.get_occupied_block(handle).is_some() {
            let free_block = must_cast(FreeBlock::new(self.free_head));
            self.memory[handle.index() as usize] = free_block;
            self.free_head = Some(handle.index().try_into().unwrap());
            true
        } else {
            false
        }
    }

    fn pop_free<'a>(&'a mut self) -> Option<NonZeroU32> {
        if let Some(idx) = self.free_head
            && let Some(block) = self.memory.get(u32::from(idx) as usize)
        {
            let new_idx = must_cast_ref::<_, FreeBlock>(block).metadata.next;

            self.free_head = new_idx;
            new_idx
        } else {
            None
        }
    }

    fn get_occupied_block(&self, handle: Handle) -> Option<&OccupiedBlock> {
        if handle.id() != 0
            && handle.index() != 0
            && let Some(block) = self.memory.get(handle.index() as usize)
            && block.metadata.id == handle.id()
        {
            Some(must_cast_ref(block))
        } else {
            None
        }
    }

    fn get_occupied_block_mut(
        &mut self,
        handle: Handle,
    ) -> Option<&mut OccupiedBlock> {
        if handle.id() != 0
            && handle.index() != 0
            && let Some(block) = self.memory.get_mut(handle.index() as usize)
            && block.metadata.id == handle.id()
        {
            Some(must_cast_mut(block))
        } else {
            None
        }
    }
}

impl Default for Allocator {
    fn default() -> Self {
        Self {
            memory: Vec::from([UnknownBlock::new(0)]),
            free_head: None,
            next_id: NonZero::new(1).unwrap(),
        }
    }
}
