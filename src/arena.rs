use crate::*;
use std::fmt::{Display, Formatter, Result};

pub trait Arena {
    type Allocator;
}

#[macro_export]
macro_rules! fixed_arena {
    ($arena:ty) => {
        impl $crate::Arena for $arena {
            type Allocator = $crate::FixedAllocator<Self>;
        }
    };
}

#[macro_export]
macro_rules! dynamic_arena {
    ($arena:ty) => {
        impl $crate::Arena for $arena {
            type Allocator = $crate::DynamicAllocator<Self>;
        }
    };
}

pub trait DisplayEntity: Sized {
    fn fmt_entity<I: ValidId<Self>>(&self, id: I, f: &mut Formatter) -> Result;
}

pub struct Entity<'a, A, I> {
    pub arena: &'a A,
    pub id: I,
}

impl<A: Arena + DisplayEntity, I: ValidId<A> + Copy> Display for Entity<'_, A, I> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.arena.fmt_entity(self.id, f)
    }
}
