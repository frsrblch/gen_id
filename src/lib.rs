use force_derive::*;
use nonmax::NonMaxU32;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::num::NonZeroU16;
use std::ops::Index;

pub use fxhash::{FxHashMap, FxHashSet};

pub mod component;
pub mod id_map;
pub mod links;

pub trait Entity: Sized {
    type IdType: IdType;
}

pub trait IdType {
    type Gen: GenTrait;
    type AllocGen: AllocGenTrait;
}

pub struct Fixed;

impl IdType for Fixed {
    type Gen = ();
    type AllocGen = ();
}

pub struct Dynamic;

impl IdType for Dynamic {
    type Gen = Gen;
    type AllocGen = u64;
}

pub trait GenTrait: std::fmt::Debug + Copy + Eq + std::hash::Hash {
    fn first() -> Self;
    fn next(self) -> Self;
    fn u64(self) -> u64;
}

impl GenTrait for () {
    fn first() -> Self {}

    fn next(self) {}

    fn u64(self) -> u64 {
        0
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Gen(NonZeroU16);

impl GenTrait for Gen {
    fn first() -> Self {
        Self(NonZeroU16::new(1).unwrap())
    }

    #[must_use]
    fn next(self) -> Self {
        NonZeroU16::new(self.0.get() + 1)
            .map(Self)
            .unwrap_or_else(Self::first)
    }

    fn u64(self) -> u64 {
        self.0.get() as u64
    }
}

pub trait AllocGenTrait: std::fmt::Debug + Default + Copy + Eq {
    fn increment<E: Entity>(&mut self, id: Id<E>);
}

impl AllocGenTrait for () {
    fn increment<E: Entity>(&mut self, _: Id<E>) {}
}

impl AllocGenTrait for u64 {
    fn increment<E: Entity>(&mut self, id: Id<E>) {
        *self <<= 1;
        *self ^= (id.index.get() as u64) << 32 | id.gen.u64()
    }
}

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

impl<E: Entity> std::hash::Hash for Id<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.gen.hash(state);
    }
}

impl<E: Entity> Id<E> {
    fn new(index: u32, gen: GenType<E>) -> Self {
        debug_assert_ne!(index, u32::MAX);

        let index = unsafe { NonMaxU32::new_unchecked(index) };

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

#[cfg(test)]
mod id_test {
    use super::{Dynamic, Entity, Fixed, Id};

    #[test]
    fn id_sizes() {
        #[derive(Debug)]
        struct F;
        impl Entity for F {
            type IdType = Fixed;
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
        let gen = Gen::first();
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

    pub fn before(&self) -> &AllocGen<E> {
        &self.before
    }

    pub fn after(&self) -> &AllocGen<E> {
        &self.after
    }
}

#[derive(Debug, ForceDefault)]
pub struct RangeAllocator<E> {
    next: u32,
    marker: PhantomData<*const E>,
}

impl<E: Entity<IdType = Fixed>> RangeAllocator<E> {
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

impl<E: Entity<IdType = Fixed>> From<Id<E>> for IdRange<E> {
    fn from(id: Id<E>) -> Self {
        let start = id.index.get();
        let end = start + 1;
        IdRange::new(start, end)
    }
}

impl<E: Entity<IdType = Fixed>> IdRange<E> {
    pub(crate) fn new(start: u32, end: u32) -> Self {
        Self {
            start,
            end,
            marker: PhantomData,
        }
    }

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

impl<E: Entity<IdType = Fixed>> IntoIterator for IdRange<E> {
    type Item = Id<E>;
    type IntoIter = RangeIter<E>;

    fn into_iter(self) -> Self::IntoIter {
        RangeIter::new(self.range())
    }
}

impl<E: Entity<IdType = Fixed>> IntoIterator for &IdRange<E> {
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

impl<E: Entity<IdType = Fixed>> Iterator for RangeIter<E> {
    type Item = Id<E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|i| Id::new(i, ()))
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
        let ptr = value as *const T as *const Self;
        unsafe { &*ptr }
    }

    pub fn new_mut(value: &mut T) -> &mut Self {
        let ptr = value as *mut T as *mut Self;
        unsafe { &mut *ptr }
    }

    pub fn as_ref(&self) -> Valid<'v, &T> {
        Valid::new(&self.value)
    }
}

impl<E: Entity<IdType = Fixed>> ValidId for Id<E> {
    type Entity = E;
    fn id(self) -> Id<E> {
        self
    }
}

impl<E: Entity<IdType = Fixed>> ValidId for &Id<E> {
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
    assert_eq!(Gen::first(), id0.gen);

    let id1 = alloc.create().value;
    assert_eq!(1, id1.index.get());
    assert_eq!(Gen::first(), id1.gen);

    assert!(alloc.kill(id1)); // first it's alive
    assert!(!alloc.kill(id1)); // then it's not

    let id2 = alloc.create().value;
    assert_eq!(1, id2.index.get());
    assert_eq!(Gen::first().next(), id2.gen);

    assert_ne!(id1.gen, id2.gen); // ensure that gen is incrementing
}

#[test]
fn append_empty() {
    #[derive(Debug)]
    struct F;
    impl Entity for F {
        type IdType = Fixed;
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
        type IdType = Fixed;
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
        type IdType = Fixed;
    }

    let mut range = IdRange::<F>::new(0, 1);

    let id = Id::new(1, ());

    range.append(id);

    assert_eq!(IdRange::new(0, 2), range);
}
