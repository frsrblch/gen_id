use crate::id::Id;
use crate::{entity::IdType, Dynamic, Entity};
use force_derive::{ForceClone, ForceDefault, ForceEq, ForcePartialEq};
use std::marker::PhantomData;
use std::num::NonZeroU16;

/// Tracks the generation of dynamic entity Ids,
/// allowing Ids that share the same index to be differentiated.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Gen(NonZeroU16);

impl Gen {
    pub const MIN: Self = unsafe { Self(NonZeroU16::new_unchecked(1)) };

    #[must_use]
    pub fn next(self) -> Self {
        NonZeroU16::new(self.0.get().wrapping_add(1))
            .map(Self)
            .unwrap_or(Self::MIN)
    }
}

/// A running checksum of IDs that have been killed.
///
/// If two `AllocGen<E>` are equal, they have seen the same IDs killed (and also in the same order).
///
/// If a collection of IDs can only have valid IDs added to it,
/// and the allocator and collection agree on which IDs have been killed,
/// and the logic of removing killed IDs from a collection is correct,
/// then an entire collection of IDs can be known to be valid
#[derive(Debug, ForceDefault, ForceClone, ForceEq, ForcePartialEq)]
pub struct AllocGen<E: Entity> {
    value: <<E as Entity>::IdType as IdType>::AllocGen,
    marker: PhantomData<E>,
}

impl<E: Entity<IdType = Dynamic>> AllocGen<E> {
    pub(crate) fn increment(&mut self, id: Id<E>) {
        let mut hasher = crc32fast::Hasher::new_with_initial(self.value);
        hasher.update(&id.index.get().to_ne_bytes());
        hasher.update(&id.gen.0.get().to_ne_bytes());
        self.value = hasher.finalize();
    }
}

#[cfg(test)]
mod tests {
    use crate::gen::{AllocGen, Gen};
    use crate::id::Id;
    use crate::tests::Dyn;
    use std::num::NonZeroU16;

    #[test]
    fn gen_next() {
        let first = Gen::MIN;
        let last = Gen(NonZeroU16::new(u16::MAX).unwrap());

        assert_ne!(first, first.next());
        assert_ne!(first, last);
        assert_eq!(first, last.next()); // wraps around to first
    }

    #[test]
    fn alloc_gen_increment() {
        let mut alloc_gen = AllocGen::<Dyn>::default();

        let mut synchronized = AllocGen::<Dyn>::default();
        let mut out_of_order = AllocGen::<Dyn>::default();
        let unsynchronized = AllocGen::<Dyn>::default();

        let id0 = Id::new(0, Gen::MIN);
        let id1 = Id::new(1, Gen::MIN);

        alloc_gen.increment(id0);
        alloc_gen.increment(id1);

        synchronized.increment(id0);
        synchronized.increment(id1);

        out_of_order.increment(id1);
        out_of_order.increment(id0);

        assert_eq!(alloc_gen, synchronized);
        assert_ne!(alloc_gen, unsynchronized);
        assert_ne!(alloc_gen, out_of_order);
    }
}
