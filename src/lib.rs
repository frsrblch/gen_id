#![feature(type_alias_impl_trait, generic_associated_types)]

use force_derive::*;
use iter_context::ContextualIterator;
use nonmax::NonMaxU32;
use ref_cast::RefCast;
use std::cmp::Ordering;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::num::NonZeroU16;
use std::ops::Index;

pub mod component;
pub mod id_map;
pub mod link;
pub mod links;
pub mod relations;

pub mod hash {
    pub use fxhash::FxHashMap as HashMap;
    pub use fxhash::FxHashSet as HashSet;
}

pub trait Insert<Key> {
    type Value;
    fn insert(&mut self, key: Key, value: Self::Value);
}

pub trait Remove<Key> {
    type Value;
    fn remove(&mut self, key: &Key) -> Option<Self::Value>;
}

pub trait Get<Key> {
    type Value;
    fn get(&self, key: Key) -> Option<&Self::Value>;
}

pub trait GetMut<Key> {
    type Value;
    fn get_mut(&mut self, key: Key) -> Option<&mut Self::Value>;
}

pub trait Entity {
    type IdType: IdType;
}

pub trait IdType {
    type Gen: GenTrait;
    type AllocGen: AllocGenTrait;
}

pub struct Static;

impl IdType for Static {
    type Gen = ();
    type AllocGen = ();
}

pub struct Dynamic;

impl IdType for Dynamic {
    type Gen = Gen;
    type AllocGen = u32;
}

pub trait GenTrait: std::fmt::Debug + Copy + Eq + std::hash::Hash + Ord {
    const MIN: Self;
    const MAX: Self;
    type BYTES: AsRef<[u8]>;
    #[must_use]
    fn next(self) -> Self;
    fn bytes(&self) -> Self::BYTES;
}

impl GenTrait for () {
    const MIN: Self = ();
    const MAX: Self = ();

    type BYTES = [u8; 0];

    fn next(self) {}

    fn bytes(&self) -> [u8; 0] {
        []
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Gen(NonZeroU16);

impl GenTrait for Gen {
    const MIN: Self = unsafe { Self(NonZeroU16::new_unchecked(1)) };
    const MAX: Self = unsafe { Self(NonZeroU16::new_unchecked(u16::MAX)) };

    type BYTES = [u8; 2];

    fn next(self) -> Self {
        NonZeroU16::new(self.0.get() + 1)
            .map(Self)
            .unwrap_or(Self::MIN)
    }

    fn bytes(&self) -> [u8; 2] {
        self.0.get().to_ne_bytes()
    }
}

pub trait AllocGenTrait: std::fmt::Debug + Default + Copy + Eq {
    fn increment<E: Entity>(&mut self, id: Id<E>);
}

impl AllocGenTrait for () {
    fn increment<E: Entity>(&mut self, _: Id<E>) {}
}

impl AllocGenTrait for u32 {
    fn increment<E: Entity>(&mut self, id: Id<E>) {
        let mut hasher = crc32fast::Hasher::new_with_initial(*self);
        hasher.update(&id.index.get().to_le_bytes());
        hasher.update(id.gen.bytes().as_ref());
        *self = hasher.finalize();
    }
}

/// A running checksum of `Id<E>` that have been killed.
///
/// If two `AllocGen<E>` are equal, they have seen the same `Id<E>` killed in the same order.
///
/// If only Valid<Id<E>> can be added to the collection, the `Allocator<E>` and collection
/// of `Id<E>` agree on which `Id<E>` have been killed, and the logic of removing killed `Id<E>`
/// from a collection is correct, an entire collection of `Id<E>` can be known to be valid.
#[derive(Debug, ForceDefault, ForceClone, ForceEq, ForcePartialEq)]
pub struct AllocGen<E: Entity> {
    value: <<E as Entity>::IdType as IdType>::AllocGen,
    marker: PhantomData<*const E>,
}

impl<E: Entity> AllocGen<E> {
    #[allow(dead_code)]
    fn increment(&mut self, id: Id<E>) {
        self.value.increment(id);
    }
}

type GenType<E> = <<E as Entity>::IdType as IdType>::Gen;

#[derive(Debug, ForceCopy, ForceClone)]
pub struct Id<E: Entity> {
    index: NonMaxU32,
    gen: GenType<E>,
    marker: PhantomData<*const E>,
}

impl<E: Entity> PartialEq for Id<E> {
    fn eq(&self, rhs: &Self) -> bool {
        self.index.eq(&rhs.index) & self.gen.eq(&rhs.gen)
    }
}

impl<E: Entity> Eq for Id<E> {}

impl<E: Entity> PartialOrd for Id<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: Entity> Ord for Id<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        // the NonMax types don't reverse comparison
        other
            .index
            .cmp(&self.index)
            .then_with(|| self.gen.cmp(&other.gen))
    }
}

impl<E: Entity> std::hash::Hash for Id<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.gen.hash(state);
    }
}

impl<E: Entity> Id<E> {
    const MIN: Self = Id {
        index: unsafe { NonMaxU32::new_unchecked(0) },
        gen: <GenType<E> as GenTrait>::MIN,
        marker: PhantomData,
    };

    const MAX: Self = Id {
        index: unsafe { NonMaxU32::new_unchecked(u32::MAX - 1) },
        gen: <GenType<E> as GenTrait>::MAX,
        marker: PhantomData,
    };

    fn new(index: u32, gen: GenType<E>) -> Self {
        debug_assert_ne!(index, u32::MAX);

        let index = unsafe { NonMaxU32::new_unchecked(index) };

        Self::new_inner(index, gen)
    }

    fn new_inner(index: NonMaxU32, gen: GenType<E>) -> Self {
        Self {
            index,
            gen,
            marker: PhantomData,
        }
    }

    pub fn index(self) -> usize {
        self.index.get() as usize
    }
}

pub trait Validator<'v, E: Entity>: AsRef<AllocGen<E>> {
    fn validate(&self, id: Id<E>) -> Option<Valid<'v, Id<E>>>;
}

#[derive(Debug, ForceDefault)]
pub struct Allocator<E: Entity> {
    ids: Vec<Entry>,
    next_dead: Option<NonMaxU32>,
    gen: AllocGen<E>,
    marker: PhantomData<*const E>,
}

impl<E: Entity<IdType = Dynamic>> Allocator<E> {
    pub fn create(&mut self) -> Valid<Id<E>> {
        let id = self.reuse_index().unwrap_or_else(|| self.create_new());
        Valid::new(id)
    }

    fn create_new(&mut self) -> Id<E> {
        let index = self.ids.len() as u32;
        let gen = Gen::MIN;
        let id = Id::new(index, gen);
        self.ids.push(Entry::from(id));
        id
    }

    fn reuse_index(&mut self) -> Option<Id<E>> {
        let index = self.next_dead?.get();
        let entry = self.ids.get_mut(index as usize)?;
        match *entry {
            Entry::Dead { next_dead, gen } => {
                self.next_dead = next_dead;
                let id = Id::new(index, gen);
                *entry = Entry::from(id);
                Some(id)
            }
            Entry::Alive(_, _) => {
                panic!("Allocator::reuse_index, Entry::Alive found at dead index ")
            }
        }
    }

    pub fn kill(&mut self, id: Id<E>) -> bool {
        match self.ids.get_mut(id.index()) {
            Some(entry) => match entry.id() {
                Some(living) => {
                    if id == living {
                        #[cfg(debug_assertions)]
                        self.gen.increment(id);

                        *entry = Entry::Dead {
                            next_dead: self.next_dead,
                            gen: id.gen.next(),
                        };
                        self.next_dead = Some(id.index);
                        true
                    } else {
                        false
                    }
                }
                None => false,
            },
            None => false,
        }
    }

    /// Drains the Vec, kills all the Ids, and filters out any duplicate or invalid Ids
    /// Returns a Killed type for the purpose of notifying other arenas of their deletion
    #[must_use]
    pub fn kill_multiple(&mut self, ids: &mut Vec<Id<E>>) -> Killed<E> {
        // Take gen value before any Ids are killed
        #[cfg(debug_assertions)]
        let start = self.gen.clone();

        // Filters out dead Ids and any duplicate values
        let ids = ids.drain(..).filter(|id| self.kill(*id)).collect();

        // Take gen value after Ids are killed
        #[cfg(debug_assertions)]
        let end = self.gen.clone();

        Killed {
            ids: Valid::new(ids),

            #[cfg(debug_assertions)]
            before: start,
            #[cfg(debug_assertions)]
            after: end,
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = Valid<Id<E>>> {
        self.ids.iter().filter_map(Entry::id).map(Valid::new)
    }

    pub fn is_alive(&self, id: Id<E>) -> bool {
        if let Some(Entry::Alive(_, gen)) = self.ids.get(id.index()) {
            id.gen.eq(gen)
        } else {
            false
        }
    }

    pub fn validate(&self, id: Id<E>) -> Option<Valid<Id<E>>> {
        self.is_alive(id).then(|| Valid::new(id))
    }

    pub fn create_only(&mut self) -> &mut CreateOnly<E> {
        CreateOnly::ref_cast_mut(self)
    }
}

impl<'v, E: Entity<IdType = Dynamic>> Validator<'v, E> for &'v Allocator<E> {
    fn validate(&self, id: Id<E>) -> Option<Valid<'v, Id<E>>> {
        Allocator::validate(self, id)
    }
}

impl<E: Entity> AsRef<AllocGen<E>> for Allocator<E> {
    fn as_ref(&self) -> &AllocGen<E> {
        &self.gen
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Entry {
    // Does not contain an Id so that the size is 8 instead of 12
    Alive(NonMaxU32, Gen),
    Dead {
        next_dead: Option<NonMaxU32>,
        gen: Gen,
    },
}

impl<E: Entity<IdType = Dynamic>> From<Id<E>> for Entry {
    fn from(id: Id<E>) -> Self {
        let Id {
            index,
            gen,
            marker: _,
        } = id;
        Entry::Alive(index, gen)
    }
}

impl Entry {
    fn id<E: Entity<IdType = Dynamic>>(&self) -> Option<Id<E>> {
        if let &Entry::Alive(index, gen) = self {
            Some(Id {
                index,
                gen,
                marker: PhantomData,
            })
        } else {
            None
        }
    }
}

#[test]
fn entry_size() {
    assert_eq!(8, std::mem::size_of::<Entry>());
}

#[repr(transparent)]
#[derive(Debug, RefCast)]
pub struct CreateOnly<'v, E: Entity> {
    alloc: Allocator<E>,
    marker: PhantomData<&'v ()>,
}

impl<'v, E: Entity<IdType = Dynamic>> CreateOnly<'v, E> {
    pub fn create(&mut self) -> Valid<'v, Id<E>> {
        Valid::new(self.alloc.create().value)
    }

    pub fn is_alive(&self, id: Id<E>) -> bool {
        self.alloc.is_alive(id)
    }

    pub fn validate(&self, id: Id<E>) -> Option<Valid<'v, Id<E>>> {
        self.alloc.validate(id).map(|v| Valid::new(v.value))
    }

    pub fn ids(&self) -> impl Iterator<Item = Valid<'v, Id<E>>> + '_ {
        self.alloc.ids().map(|v| Valid::new(v.value))
    }
}

impl<'v, E: Entity<IdType = Dynamic>> Validator<'v, E> for &CreateOnly<'v, E> {
    fn validate(&self, id: Id<E>) -> Option<Valid<'v, Id<E>>> {
        CreateOnly::validate(self, id)
    }
}

impl<'v, E: Entity<IdType = Dynamic>> Validator<'v, E> for &mut CreateOnly<'v, E> {
    fn validate(&self, id: Id<E>) -> Option<Valid<'v, Id<E>>> {
        CreateOnly::validate(self, id)
    }
}

impl<E: Entity> AsRef<AllocGen<E>> for CreateOnly<'_, E> {
    fn as_ref(&self) -> &AllocGen<E> {
        self.alloc.as_ref()
    }
}

/// A list of valid, unique Ids that have been killed.
/// Includes before and after allocator generations for validating and updating AllocGen values  
pub struct Killed<'v, E: Entity> {
    ids: Valid<'v, Vec<Id<E>>>,

    #[cfg(debug_assertions)]
    before: AllocGen<E>,
    #[cfg(debug_assertions)]
    after: AllocGen<E>,
}

impl<'v, E: Entity> Killed<'v, E> {
    pub fn ids(&self) -> &Valid<'v, Vec<Id<E>>> {
        &self.ids
    }

    #[cfg(debug_assertions)]
    pub fn before(&self) -> &AllocGen<E> {
        &self.before
    }

    #[cfg(debug_assertions)]
    pub fn after(&self) -> &AllocGen<E> {
        &self.after
    }

    #[cfg(debug_assertions)]
    pub fn nofity<Collection: Kill<E> + AsRef<AllocGen<E>>>(&self, collection: &mut Collection) {
        assert!(self.before() == collection.as_ref());

        for id in self.ids() {
            collection.kill(id);
        }

        assert!(self.after() == collection.as_ref());
    }

    #[cfg(not(debug_assertions))]
    pub fn nofity<Collection: Kill<E>>(&self, collection: &mut Collection) {
        for id in self.ids() {
            collection.kill(id);
        }
    }
}

pub trait Kill<E> {
    fn kill<V: ValidId<Entity = E>>(&mut self, id: V);
}

#[derive(Debug, ForceDefault)]
pub struct RangeAllocator<E> {
    next: u32,
    marker: PhantomData<*const E>,
}

impl<E: Entity<IdType = Static>> RangeAllocator<E> {
    pub fn create(&mut self) -> Id<E> {
        let id = Id::new(self.next, ());
        self.next += 1;
        id
    }

    pub fn create_range(&mut self, count: usize) -> IdRange<E> {
        let start = self.next;
        let end = start + count as u32;
        self.next = end;
        IdRange::new(start, end)
    }
}

pub trait ValidId: Copy {
    type Entity: Entity;
    fn id(self) -> Id<Self::Entity>;
}

#[derive(Debug, ForceDefault, ForceCopy, ForceClone, ForceEq, ForcePartialEq, ForceHash)]
pub struct IdRange<E> {
    start: u32,
    end: u32,
    marker: PhantomData<*const E>,
}

impl<E: Entity<IdType = Static>> Insert<()> for IdRange<E> {
    type Value = Id<E>;

    fn insert(&mut self, _: (), value: Self::Value) {
        self.append(value);
    }
}

impl<E: Entity<IdType = Static>> From<Id<E>> for IdRange<E> {
    fn from(id: Id<E>) -> Self {
        let start = id.index.get();
        let end = start + 1;
        IdRange::new(start, end)
    }
}

impl<E: Entity<IdType = Static>> IdRange<E> {
    pub(crate) fn new(start: u32, end: u32) -> Self {
        Self {
            start,
            end,
            marker: PhantomData,
        }
    }

    #[track_caller]
    pub fn append(&mut self, id: Id<E>) {
        if self.end == id.index.get() {
            self.end += 1;
        } else if self.is_empty() {
            *self = id.into();
        } else {
            panic!("IdRange::append: id has invalid index")
        }
    }

    pub fn position(&self, id: Id<E>) -> Option<usize> {
        let index = id.index.get();
        if index < self.end {
            index.checked_sub(self.start).map(|i| i as usize)
        } else {
            None
        }
    }

    fn range(&self) -> std::ops::Range<u32> {
        self.start..self.end
    }

    pub fn contains(&self, id: Id<E>) -> bool {
        self.range().contains(&id.index.get())
    }

    pub fn len(&self) -> usize {
        self.range().len()
    }

    pub fn is_empty(&self) -> bool {
        self.range().is_empty()
    }
}

impl<E: Entity<IdType = Static>> IntoIterator for IdRange<E> {
    type Item = Id<E>;
    type IntoIter = RangeIter<E>;

    fn into_iter(self) -> Self::IntoIter {
        RangeIter::new(self.range())
    }
}

impl<E: Entity<IdType = Static>> IntoIterator for &IdRange<E> {
    type Item = Id<E>;
    type IntoIter = RangeIter<E>;

    fn into_iter(self) -> Self::IntoIter {
        RangeIter::new(self.range())
    }
}

#[derive(ForceClone)]
pub struct RangeIter<E> {
    range: std::ops::Range<u32>,
    marker: PhantomData<*const E>,
}

impl<E> RangeIter<E> {
    fn new(range: std::ops::Range<u32>) -> Self {
        Self {
            range,
            marker: PhantomData,
        }
    }
}

impl<E: Entity<IdType = Static>> Iterator for RangeIter<E> {
    type Item = Id<E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|i| Id::new(i, ()))
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, RefCast)]
pub struct Valid<'v, T> {
    pub value: T,
    marker: PhantomData<&'v ()>,
}

impl<T: PartialEq> PartialEq<T> for Valid<'_, T> {
    fn eq(&self, other: &T) -> bool {
        self.value.eq(other)
    }
}

impl<'v, T> Valid<'v, T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            marker: PhantomData,
        }
    }

    pub fn new_ref(value: &T) -> &Self {
        Valid::ref_cast(value)
    }

    pub fn new_mut(value: &mut T) -> &mut Self {
        Valid::ref_cast_mut(value)
    }
}

impl<E: Entity<IdType = Static>> ValidId for Id<E> {
    type Entity = E;
    fn id(self) -> Id<E> {
        self
    }
}

impl<E: Entity<IdType = Static>> ValidId for &Id<E> {
    type Entity = E;
    fn id(self) -> Id<E> {
        *self
    }
}

impl<'v, E: Entity> ValidId for Valid<'v, Id<E>> {
    type Entity = E;
    fn id(self) -> Id<E> {
        self.value
    }
}

impl<'v, E: Entity> ValidId for &Valid<'v, Id<E>> {
    type Entity = E;
    fn id(self) -> Id<E> {
        self.value
    }
}

impl<'v, E: Entity> ValidId for Valid<'v, &Id<E>> {
    type Entity = E;
    fn id(self) -> Id<E> {
        *self.value
    }
}

impl<'v, E: Entity> ValidId for &Valid<'v, &Id<E>> {
    type Entity = E;
    fn id(self) -> Id<E> {
        *self.value
    }
}

impl<'v, Ix, Indexable: Index<Ix>> Index<Ix> for Valid<'v, Indexable>
where
    <Indexable as std::ops::Index<Ix>>::Output: Sized,
{
    type Output = Valid<'v, Indexable::Output>;

    fn index(&self, index: Ix) -> &Self::Output {
        Valid::new_ref(self.value.index(index))
    }
}

impl<'v, I: IntoIterator> IntoIterator for Valid<'v, I> {
    type Item = Valid<'v, I::Item>;
    type IntoIter = ValidIter<'v, I::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        ValidIter {
            iter: self.value.into_iter(),
            marker: PhantomData,
        }
    }
}

impl<'a, 'v, I> IntoIterator for &'a Valid<'v, I>
where
    &'a I: IntoIterator,
{
    type Item = Valid<'v, <&'a I as IntoIterator>::Item>;
    type IntoIter = ValidIter<'v, <&'a I as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        ValidIter {
            iter: (&self.value).into_iter(),
            marker: PhantomData,
        }
    }
}

impl<'v, T: ContextualIterator> ContextualIterator for Valid<'v, T> {
    type Context = T::Context;
}

impl<'a, 'v, T> ContextualIterator for &'a Valid<'v, T>
where
    &'a T: ContextualIterator,
{
    type Context = <&'a T as ContextualIterator>::Context;
}

pub struct ValidIter<'v, I> {
    iter: I,
    marker: PhantomData<&'v ()>,
}

impl<'v, I: Iterator> Iterator for ValidIter<'v, I> {
    type Item = Valid<'v, I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Valid::new)
    }
}

impl<'v, T: AsRef<U>, U> AsRef<Valid<'v, U>> for Valid<'v, T> {
    fn as_ref(&self) -> &Valid<'v, U> {
        Valid::new_ref(self.value.as_ref())
    }
}

impl<'v, E: Entity> Valid<'v, (&Id<E>, &Id<E>)> {
    pub fn key(&self) -> Valid<'v, &Id<E>> {
        Valid::new(self.value.0)
    }

    pub fn value(&self) -> Valid<'v, &Id<E>> {
        Valid::new(self.value.1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn id_sizes() {
        #[derive(Debug)]
        struct F;
        impl Entity for F {
            type IdType = Static;
        }

        #[derive(Debug)]
        struct D;
        impl Entity for D {
            type IdType = Dynamic;
        }

        use std::mem::size_of;
        assert_eq!(4, size_of::<Id<F>>());
        assert_eq!(4, size_of::<Option<Id<F>>>());
        assert_eq!(8, size_of::<Id<D>>());
        assert_eq!(8, size_of::<Option<Id<D>>>());
    }

    #[test]
    fn cmp() {
        #[derive(Debug)]
        struct D;
        impl Entity for D {
            type IdType = Dynamic;
        }

        let mut alloc = Allocator::<D>::default();
        let id0 = alloc.create().value;
        let id1 = alloc.create().value;

        assert!(NonMaxU32::new(0) > NonMaxU32::new(1));
        assert!(Gen::MIN < Gen::MAX);
        assert!(Id::<D>::MIN < Id::<D>::MAX);
        assert!(id0 < id1);
    }

    #[test]
    fn id_creation() {
        #[derive(Debug)]
        struct E;
        impl Entity for E {
            type IdType = Dynamic;
        }
        let mut alloc = Allocator::<E>::default();

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
        #[derive(Debug)]
        struct E;
        impl Entity for E {
            type IdType = Dynamic;
        }

        let mut alloc = Allocator::<E>::default();
        let mut incorrect = AllocGen::<E>::default();
        let mut correct = AllocGen::<E>::default();

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
    fn append_empty() {
        #[derive(Debug)]
        struct F;
        impl Entity for F {
            type IdType = Static;
        }

        let mut range = IdRange::default();
        assert!(range.is_empty());
        let id = Id::<F>::new(2, ());

        range.append(id);
        assert_eq!(IdRange::from(id), range);
    }

    #[test]
    #[should_panic]
    fn append_given_invalid_index() {
        #[derive(Debug)]
        struct F;
        impl Entity for F {
            type IdType = Static;
        }

        let mut range = IdRange::<F>::new(0, 1);

        let id = Id::new(2, ());

        range.append(id);
    }

    #[test]
    fn range_append() {
        #[derive(Debug)]
        struct F;
        impl Entity for F {
            type IdType = Static;
        }

        let mut range = IdRange::<F>::new(0, 1);

        let id = Id::new(1, ());

        range.append(id);

        assert_eq!(IdRange::new(0, 2), range);
    }
}
