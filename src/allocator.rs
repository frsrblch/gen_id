use crate::gen::{AllocGen, Gen};
use crate::{Dynamic, Entity, Id, IdRange, Static, Valid};
use nonmax::NonMaxU32;
use ref_cast::RefCast;
use std::marker::PhantomData;

#[cfg(feature = "rayon")]
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

/// Allocates indices for dynamic Ids.
#[derive(Debug)]
pub struct Allocator<E: Entity> {
    entries: Vec<Entry>,
    next_dead: Option<NonMaxU32>,
    gen: AllocGen<E>,
    marker: PhantomData<E>,
}

impl<E: Entity> Default for Allocator<E> {
    #[inline]
    fn default() -> Self {
        Self {
            entries: Default::default(),
            next_dead: Default::default(),
            gen: Default::default(),
            marker: Default::default(),
        }
    }
}

impl<E: Entity<IdType = Dynamic>> Allocator<E> {
    #[inline]
    pub fn create(&mut self) -> Valid<Id<E>> {
        let id = self.reuse_index().unwrap_or_else(|| self.create_new());
        Valid::new(id)
    }

    fn create_new(&mut self) -> Id<E> {
        let index = self.entries.len() as u32;
        let id = Id::first(index);
        self.entries.push(Entry::from(id));
        id
    }

    fn reuse_index(&mut self) -> Option<Id<E>> {
        let index = self.next_dead?.get();
        let entry = self.entries.get_mut(index as usize)?;
        match *entry {
            Entry::Dead { next_dead, gen } => {
                self.next_dead = next_dead;
                let id = Id::new(index, gen);
                *entry = Entry::from(id);
                Some(id)
            }
            Entry::Alive { index: _, gen: _ } => {
                panic!("Allocator::reuse_index, Entry::Alive found at dead index ")
            }
        }
    }

    #[inline]
    pub fn kill(&mut self, id: Id<E>) -> bool {
        if let Some(entry) = self.entries.get_mut(id.index()) {
            if let Some(living) = entry.id() {
                if id.eq(&living) {
                    self.gen.increment(id);

                    *entry = Entry::Dead {
                        next_dead: self.next_dead,
                        gen: id.gen.next(),
                    };

                    self.next_dead = Some(id.index);

                    return true;
                }
            }
        }

        false
    }

    /// Drains the Vec, kills all the Ids, and filters out any duplicate or invalid Ids
    /// Returns a Killed type for the purpose of notifying other collections of their deletion
    #[must_use]
    #[inline]
    pub fn kill_many(&mut self, ids: &mut Vec<Id<E>>) -> KilledIds<E> {
        // Take gen value before any Ids are killed
        let before = self.gen.clone();

        // Filters out dead Ids and any duplicate values
        let ids = ids.drain(..).filter(|id| self.kill(*id)).collect();

        // Take gen value after Ids are killed
        let after = self.gen.clone();

        KilledIds {
            ids: Valid::new(ids),
            before,
            after,
        }
    }

    /// `impl Iterator<Item = Valid<Id<E>>>`
    #[inline]
    pub fn ids(&self) -> Ids<E> {
        Ids::new(&self.entries)
    }

    #[cfg(feature = "rayon")]
    #[inline]
    pub fn par_ids(&self) -> impl ParallelIterator<Item = Valid<Id<E>>> {
        self.entries
            .as_slice()
            .into_par_iter()
            .filter_map(|e| e.id())
            .map(Valid::new)
    }

    /// `impl IntoIterator<Item = Option<Valid<Id<E>>>> + [iter_context::ContextualIterator]`
    #[inline]
    pub fn sparse_ids(&self) -> SparseIds<E> {
        SparseIds::new(&self.entries)
    }

    #[inline]
    pub fn is_alive(&self, id: Id<E>) -> bool {
        if let Some(Entry::Alive { gen, .. }) = self.entries.get(id.index()) {
            id.gen.eq(gen)
        } else {
            false
        }
    }

    #[inline]
    pub fn validate(&self, id: Id<E>) -> Option<Valid<Id<E>>> {
        self.is_alive(id).then(|| Valid::new(id))
    }

    /// Guarantees that no `Id<E>` can be killed during the `'valid` lifetime. Allows `Valid<Id<E>>`
    /// to have a longer lifetime than the immediate lifetime of `&'a self` or `&'a mut self`.
    #[inline]
    pub fn create_only(&mut self) -> &mut CreateOnly<E> {
        CreateOnly::ref_cast_mut(self)
    }
}

#[derive(Debug)]
pub struct Ids<'slice, 'valid, E> {
    iter: std::slice::Iter<'slice, Entry>,
    valid: PhantomData<&'valid ()>,
    entity: PhantomData<E>,
}

impl<'slice, 'valid, E> Ids<'slice, 'valid, E> {
    #[allow(clippy::ptr_arg)] // We want the whole vec
    fn new(entries: &'slice Vec<Entry>) -> Self {
        Self {
            iter: entries.iter(),
            valid: PhantomData,
            entity: PhantomData,
        }
    }
}

impl<'slice, 'valid, E: Entity<IdType = Dynamic>> Iterator for Ids<'slice, 'valid, E> {
    type Item = Valid<'valid, Id<E>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        for entry in self.iter.by_ref() {
            if let Some(id) = entry.id() {
                return Some(Valid::new(id));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct SparseIds<'slice, 'valid, E> {
    iter: std::slice::Iter<'slice, Entry>,
    valid: PhantomData<&'valid ()>,
    entity: PhantomData<E>,
}

impl<'slice, 'valid, E> SparseIds<'slice, 'valid, E> {
    #[allow(clippy::ptr_arg)] // We want the whole vec
    fn new(entries: &'slice Vec<Entry>) -> Self {
        Self {
            iter: entries.iter(),
            valid: PhantomData,
            entity: PhantomData,
        }
    }
}

impl<'slice, 'valid, E: Entity<IdType = Dynamic>> IntoIterator for SparseIds<'slice, 'valid, E> {
    type Item = Option<Valid<'valid, Id<E>>>;
    type IntoIter = SparseIdsIter<'slice, 'valid, E>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        SparseIdsIter::new(self.iter)
    }
}

impl<'slice, 'valid, E: Entity<IdType = Dynamic>> iter_context::ContextualIterator
    for SparseIds<'slice, 'valid, E>
{
    type Context = E;
}

#[derive(Debug)]
pub struct SparseIdsIter<'slice, 'valid, E> {
    iter: std::slice::Iter<'slice, Entry>,
    valid: PhantomData<&'valid ()>,
    entity: PhantomData<E>,
}

impl<'slice, 'valid, E> SparseIdsIter<'slice, 'valid, E> {
    #[allow(clippy::ptr_arg)] // We want the whole vec
    fn new(iter: std::slice::Iter<'slice, Entry>) -> Self {
        Self {
            iter,
            valid: PhantomData,
            entity: PhantomData,
        }
    }
}

impl<'slice, 'valid, E: Entity<IdType = Dynamic>> Iterator for SparseIdsIter<'slice, 'valid, E> {
    type Item = Option<Valid<'valid, Id<E>>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Entry::id).map(|id| id.map(Valid::new))
    }
}

#[repr(transparent)]
#[derive(Debug, RefCast)]
pub struct CreateOnly<'valid, E: Entity> {
    alloc: Allocator<E>,
    marker: PhantomData<&'valid ()>,
}

impl<'valid, E: Entity<IdType = Dynamic>> CreateOnly<'valid, E> {
    #[inline]
    pub fn create(&mut self) -> Valid<'valid, Id<E>> {
        Valid::new(self.alloc.create().value)
    }

    #[inline]
    pub fn is_alive(&self, id: Id<E>) -> bool {
        self.alloc.is_alive(id)
    }

    #[inline]
    pub fn validate(&self, id: Id<E>) -> Option<Valid<'valid, Id<E>>> {
        self.alloc.validate(id).map(|v| Valid::new(v.value))
    }

    #[inline]
    pub fn ids<'slice>(&'slice self) -> Ids<'slice, 'valid, E> {
        Ids::new(&self.alloc.entries)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Entry {
    // Does not contain an Id so that the size is 8 instead of 12
    Alive {
        index: NonMaxU32,
        gen: Gen,
    },
    Dead {
        next_dead: Option<NonMaxU32>,
        gen: Gen,
    },
}

impl<E: Entity<IdType = Dynamic>> From<Id<E>> for Entry {
    fn from(id: Id<E>) -> Self {
        let Id { index, gen, .. } = id;
        Entry::Alive { index, gen }
    }
}

impl Entry {
    fn id<E: Entity<IdType = Dynamic>>(&self) -> Option<Id<E>> {
        if let &Entry::Alive { index, gen } = self {
            Some(Id::new_non_max(index, gen))
        } else {
            None
        }
    }
}

/// A list of valid, unique Ids that have been killed.
/// Includes before and after allocator generations for validating and updating AllocGen values
pub struct KilledIds<'v, E: Entity> {
    ids: Valid<'v, Vec<Id<E>>>,
    before: AllocGen<E>,
    after: AllocGen<E>,
}

impl<'v, E: Entity> KilledIds<'v, E> {
    #[inline]
    pub fn ids(&self) -> &Valid<'v, Vec<Id<E>>> {
        &self.ids
    }

    #[inline]
    pub fn before(&self) -> &AllocGen<E> {
        &self.before
    }

    #[inline]
    pub fn after(&self) -> &AllocGen<E> {
        &self.after
    }
}

/// Allocates indices for static Ids.
#[derive(Debug)]
pub struct RangeAllocator<E> {
    next: u32,
    marker: PhantomData<E>,
}

impl<E> Default for RangeAllocator<E> {
    #[inline]
    fn default() -> Self {
        Self {
            next: Default::default(),
            marker: Default::default(),
        }
    }
}

impl<E: Entity<IdType = Static>> RangeAllocator<E> {
    #[inline]
    pub fn create(&mut self) -> Id<E> {
        let id = Id::new(self.next, ());
        self.next += 1;
        id
    }

    #[inline]
    pub fn create_range(&mut self, count: usize) -> IdRange<E> {
        let start = self.next;
        let end = start + count as u32;
        self.next = end;
        IdRange::new(start, end)
    }

    #[inline]
    pub fn ids(&self) -> IdRange<E> {
        IdRange::new(0, self.next)
    }

    #[cfg(feature = "rayon")]
    #[inline]
    pub fn par_ids(&self) -> impl ParallelIterator<Item = Id<E>> + IndexedParallelIterator {
        (0..self.next).into_par_iter().map(|i| Id::new(i, ()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{Dyn, Stat};
    use crate::valid::ValidId;

    #[test]
    fn entry_size() {
        assert_eq!(8, std::mem::size_of::<Entry>());
    }

    #[test]
    fn allocator_id_gen() {
        let mut alloc = Allocator::<Dyn>::default();

        let id00 = alloc.create().id();
        alloc.kill(id00);
        let id01 = alloc.create().id();
        let id10 = alloc.create().id();

        assert_eq!(Id::new(0, Gen::MIN), id00);
        assert_eq!(Id::new(0, Gen::MIN.next()), id01);
        assert_eq!(Id::new(1, Gen::MIN), id10);
    }

    #[test]
    fn allocator_kill() {
        let mut alloc = Allocator::<Dyn>::default();
        let id = alloc.create().id();

        assert!(alloc.kill(id));
        assert!(!alloc.kill(id));
    }

    #[test]
    fn allocator_create_returns_incremented_index() {
        let mut alloc = Allocator::<Dyn>::default();
        let id0 = alloc.create().id();
        let id1 = alloc.create().id();

        assert_eq!(0, id0.index());
        assert_eq!(1, id1.index());
        assert_eq!(id0.gen, id1.gen);
    }

    #[test]
    fn allocator_reuse_index_increments_gen() {
        let mut alloc = Allocator::<Dyn>::default();

        let id00 = alloc.create().id();
        alloc.kill(id00);
        let id01 = alloc.create().id();

        assert_eq!(Id::new(0, Gen::MIN), id00);
        assert_eq!(Id::new(0, Gen::MIN.next()), id01);
    }

    #[test]
    fn allocator_ids() {
        let mut alloc = Allocator::<Dyn>::default();

        let id0 = alloc.create().id();
        let id1 = alloc.create().id();
        let id2 = alloc.create().id();
        let id3 = alloc.create().id();

        alloc.kill(id1);

        let ids = alloc.ids().collect::<Vec<_>>();

        // tests PartialEq for both Valid<Id> and Id
        assert_eq!(&ids, &vec![id0, id2, id3]);
        assert_eq!(&vec![id0, id2, id3], &ids);
    }

    #[test]
    fn allocator_is_alive() {
        let mut alloc = Allocator::<Dyn>::default();
        let id00 = alloc.create().id();
        let id10 = alloc.create().id();
        alloc.kill(id00);
        let id01 = alloc.create().id();

        assert!(!alloc.is_alive(id00));
        assert!(alloc.is_alive(id10));
        assert!(alloc.is_alive(id01));
    }

    #[test]
    fn allocator_validate() {
        let mut alloc = Allocator::<Dyn>::default();
        let id00 = alloc.create().id();
        let id10 = alloc.create().id();
        alloc.kill(id00);

        let valid0 = alloc.validate(id00);
        assert!(valid0.is_none());

        let valid1 = alloc.validate(id10);
        assert_eq!(valid1.unwrap(), id10);
    }

    #[test]
    fn create_only_is_alive() {
        let mut alloc = Allocator::<Dyn>::default();
        let id00 = alloc.create().id();
        let id10 = alloc.create().id();
        alloc.kill(id00);
        let id01 = alloc.create().id();
        let alloc = alloc.create_only();

        assert!(!alloc.is_alive(id00));
        assert!(alloc.is_alive(id10));
        assert!(alloc.is_alive(id01));
    }

    #[test]
    fn create_only_validate() {
        let mut alloc = Allocator::<Dyn>::default();
        let id00 = alloc.create().id();
        let id10 = alloc.create().id();
        alloc.kill(id00);
        let alloc = alloc.create_only();

        let valid0 = alloc.validate(id00);
        assert!(valid0.is_none());

        let valid1 = alloc.validate(id10);
        assert_eq!(valid1.unwrap(), id10);
    }

    #[test]
    fn range_alloc_create() {
        let mut alloc = RangeAllocator::<Stat>::default();
        let id0 = Id::<Stat>::new(0, ());
        let id1 = Id::<Stat>::new(1, ());

        assert_eq!(id0, alloc.create());
        assert_eq!(id1, alloc.create());
    }

    #[test]
    fn range_alloc_create_range() {
        let mut alloc = RangeAllocator::<Stat>::default();
        let ids = alloc.create_range(2);
        let mut iter = ids.into_iter();
        let id0 = Id::<Stat>::new(0, ());
        let id1 = Id::<Stat>::new(1, ());

        assert_eq!(Some(id0), iter.next());
        assert_eq!(Some(id1), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn range_alloc_ids() {
        let mut alloc = RangeAllocator::<Stat>::default();
        let range = alloc.create_range(2);
        let ids = alloc.ids();

        assert_eq!(range, ids);
    }

    #[test]
    fn id_creation() {
        let mut alloc = Allocator::<Dyn>::default();

        let id0 = alloc.create().value;
        assert_eq!(0, id0.index.get());
        assert_eq!(Gen::MIN, id0.gen);

        let id1 = alloc.create().value;
        assert_eq!(1, id1.index.get());
        assert_eq!(Gen::MIN, id1.gen);

        assert!(alloc.kill(id1)); // first it's alive
        assert!(!alloc.kill(id1)); // then it's not

        let id2 = alloc.create().value;
        assert_eq!(1, id2.index.get());
        assert_eq!(Gen::MIN.next(), id2.gen);

        assert_ne!(id1.gen, id2.gen); // ensure that gen is incrementing
    }

    #[test]
    fn increment_gen() {
        let mut alloc = Allocator::<Dyn>::default();
        let mut incorrect = AllocGen::<Dyn>::default();
        let mut correct = AllocGen::<Dyn>::default();

        // skip incrementing gen to simulate an error
        let id = alloc.create().value;
        alloc.kill(id);
        correct.increment(id);

        // do a large number properly
        for _ in 0..256 {
            // make sure we aren't just using index 0 the whole time
            let _ = alloc.create();

            let id = alloc.create().value;
            alloc.kill(id);
            correct.increment(id);
            incorrect.increment(id);
        }

        // checksums should match because both see same ids killed
        assert_eq!(correct, alloc.gen);
        // checksums should not match because the first id was skipped
        assert_ne!(incorrect, alloc.gen);
    }

    #[test]
    fn kill_many_removes_duplicates() {
        let mut alloc = Allocator::<Dyn>::default();
        let id0 = alloc.create().id();

        let mut kill = vec![id0, id0];

        let kill_many = alloc.kill_many(&mut kill);

        assert_eq!(kill_many.ids.value, vec![id0]);
    }

    #[test]
    fn kill_many_removes_dead() {
        let mut alloc = Allocator::<Dyn>::default();
        let id0 = alloc.create().id();
        alloc.kill(id0);

        let mut kill = vec![id0];

        let kill_many = alloc.kill_many(&mut kill);

        assert!(kill_many.ids.value.is_empty());
    }

    #[test]
    fn allocator_sparse_ids() {
        let mut alloc = Allocator::<Dyn>::default();
        let id0 = alloc.create().id();
        let id1 = alloc.create().id();
        let id2 = alloc.create().id();
        alloc.kill(id0);

        let ids = alloc.sparse_ids().into_iter().collect::<Vec<_>>();

        assert_eq!(
            vec![None, Some(Valid::new(id1)), Some(Valid::new(id2))],
            ids
        );
    }
}
