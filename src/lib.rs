extern crate core;

pub mod allocator;
pub mod component;
pub mod entity;
pub mod gen;
mod id;
pub mod relations;
mod valid;

pub use allocator::{Allocator, RangeAllocator};
pub use component::Component;
pub use entity::{Dynamic, Entity, Static};
pub use id::{Id, IdRange};
pub use iter_context::{ContextualIterator, FromContextualIterator};
pub use valid::{Valid, ValidId};

#[cfg(test)]
pub mod tests {
    use crate::{Dynamic, Entity, Static};

    #[derive(Debug)]
    pub struct Dyn;

    impl Entity for Dyn {
        type IdType = Dynamic;
    }

    #[derive(Debug)]
    pub struct Stat;

    impl Entity for Stat {
        type IdType = Static;
    }
}
