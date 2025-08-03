#![no_std]
#![feature(sync_unsafe_cell)]
#![feature(ptr_as_ref_unchecked)]

use core::ops::{Deref, DerefMut};

pub mod sys;
use sys::{
    PIPECLEANER_get_entity,
    PIPECLEANER_create_entity,
    PIPECLEANER_remove_entity,
    PIPECLEANER_write_entity_back,
};

use pipe_cleaner_shared as shared;
pub use shared::{EngineFields, PipePosition};
use bytemuck::{Zeroable, Pod, cast_mut, cast_ref};

pub const GAME_FIELDS_SZ: usize =
    shared::ENTITY_SZ - size_of::<shared::EngineFields>() / shared::FIELD_SZ;

#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Entity<T: Pod> {
    pub engine_fields: shared::EngineFields,
    pub game_fields: T,
}

pub struct EntityRef<T: Pod> {
    handle: u64,
    inner: Entity<T>,
}

impl<T: Pod> EntityRef<T> {
    pub fn spawn() -> Self {
        let handle = unsafe { PIPECLEANER_create_entity() };

        Self {
            handle,
            inner: Zeroable::zeroed()
        }
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn from_handle(handle: u64) -> Option<Self> {
        let mut entity = Zeroable::zeroed();
        
        let failure_code = unsafe {
            PIPECLEANER_get_entity(handle, cast_mut(&mut entity) as _)
        };

        if failure_code == 0 {
            None
        } else {
            Some( Self {
                handle,
                inner: entity,
            })
        }
    }

    pub fn remove(self) {
        unsafe {
            PIPECLEANER_remove_entity(self.handle);
        }
    }
}

impl<T: Pod> Deref for EntityRef<T> {
    type Target = Entity<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Pod> DerefMut for EntityRef<T> {
    fn deref_mut(&mut self) -> &mut Entity<T> {
        &mut self.inner
    }
}

impl<T: Pod> Drop for EntityRef<T> {
    fn drop(&mut self) {
        unsafe {
            PIPECLEANER_write_entity_back(
                self.handle,
                cast_ref(&**self) as _
            );
        }
    }
}
