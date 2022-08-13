use crate::{entity::IdType, Dynamic, Entity, Static};
use force_derive::{ForceClone, ForceDefault, ForceEq, ForceHash, ForcePartialEq};
use nonmax::NonMaxU32;
use std::cmp::Ordering;
use std::iter::FusedIterator;
use std::marker::PhantomData;

type GenType<E> = <<E as Entity>::IdType as IdType>::Gen;

#[derive(Debug, ForcePartialEq, ForceEq, ForceHash)]
pub struct Id<E: Entity> {
    pub(crate) index: NonMaxU32,
    pub(crate) gen: GenType<E>,
    marker: PhantomData<E>,
}

unsafe impl<E: Entity> Send for Id<E>
where
    GenType<E>: Send,
    NonMaxU32: Send,
{
}

unsafe impl<E: Entity> Sync for Id<E>
where
    GenType<E>: Sync,
    NonMaxU32: Sync,
{
}

impl<E: Entity> Clone for Id<E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: Entity> Copy for Id<E> {}

impl<'v, E: Entity> PartialEq<crate::Valid<'v, Id<E>>> for Id<E> {
    #[inline]
    fn eq(&self, other: &crate::Valid<'v, Id<E>>) -> bool {
        self.eq(&other.value)
    }
}

impl<E: Entity> PartialEq for Id<E> {
    fn eq(&self, other: &Self) -> bool {
        self.index.eq(&other.index) & self.gen.eq(&other.gen)
    }
}

impl<E: Entity> Eq for Id<E> {}

impl<E: Entity> PartialOrd for Id<E> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: Entity> Ord for Id<E> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // the NonMax types don't reverse comparison
        other
            .index
            .cmp(&self.index)
            .then_with(|| self.gen.cmp(&other.gen))
    }
}

impl<E: Entity<IdType = Dynamic>> Id<E> {
    pub(crate) fn first(index: u32) -> Self {
        Self::new(index, crate::gen::Gen::MIN)
    }
}

impl<E: Entity> Id<E> {
    #[cfg(test)]
    pub fn new(index: u32, gen: GenType<E>) -> Self {
        let index = NonMaxU32::new(index).expect("index out of range");
        Self::new_non_max(index, gen)
    }

    #[cfg(not(test))]
    pub(crate) fn new(index: u32, gen: GenType<E>) -> Self {
        let index = NonMaxU32::new(index).expect("index out of range");
        Self::new_non_max(index, gen)
    }

    pub(crate) fn new_non_max(index: NonMaxU32, gen: GenType<E>) -> Self {
        Self {
            index,
            gen,
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn index(self) -> usize {
        self.index.get() as usize
    }
}

#[derive(Debug, ForceDefault, ForceEq, ForcePartialEq, ForceHash)]
pub struct IdRange<E> {
    start: u32,
    end: u32,
    marker: PhantomData<E>,
}

unsafe impl<E> Send for IdRange<E> {}
unsafe impl<E> Sync for IdRange<E> {}

impl<E> Clone for IdRange<E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<E> Copy for IdRange<E> {}

impl<E: Entity<IdType = Static>> From<Id<E>> for IdRange<E> {
    #[inline]
    fn from(id: Id<E>) -> Self {
        let start = id.index.get();
        let end = start + 1;
        IdRange::new(start, end)
    }
}

impl<E: Entity<IdType = Static>> IdRange<E> {
    #[cfg(test)]
    pub fn new(start: u32, end: u32) -> Self {
        Self {
            start,
            end,
            marker: PhantomData,
        }
    }

    #[cfg(not(test))]
    pub(crate) fn new(start: u32, end: u32) -> Self {
        Self {
            start,
            end,
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn contains(&self, id: Id<E>) -> bool {
        self.range().contains(&id.index.get())
    }

    #[inline]
    fn range(&self) -> std::ops::Range<u32> {
        self.start..self.end
    }

    #[inline]
    pub fn range_usize(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.range().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range().is_empty()
    }
}

impl<E: Entity<IdType = Static>> IntoIterator for IdRange<E> {
    type Item = Id<E>;
    type IntoIter = RangeIter<E>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        RangeIter::new(self.range())
    }
}

impl<E: Entity<IdType = Static>> IntoIterator for &IdRange<E> {
    type Item = Id<E>;
    type IntoIter = RangeIter<E>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        RangeIter::new(self.range())
    }
}

// #[cfg(feature = "rayon")]
// impl<E: Entity<IdType = Static> + Send> IntoParallelIterator for &IdRange<E> {
//     type Iter = RangeIter<E>;
//     type Item = Id<E>;
//
//     fn into_par_iter(self) -> Self::Iter {
//         self.into_iter()
//     }
// }

#[derive(ForceClone)]
pub struct RangeIter<E> {
    range: std::ops::Range<u32>,
    marker: PhantomData<E>,
}

impl<E> RangeIter<E> {
    #[inline]
    fn new(range: std::ops::Range<u32>) -> Self {
        Self {
            range,
            marker: PhantomData,
        }
    }
}

impl<E: Entity<IdType = Static>> Iterator for RangeIter<E> {
    type Item = Id<E>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|i| Id::new(i, ()))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<E: Entity<IdType = Static>> DoubleEndedIterator for RangeIter<E> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range.next_back().map(|i| Id::new(i, ()))
    }
}

impl<E: Entity<IdType = Static>> ExactSizeIterator for RangeIter<E> {}

impl<E: Entity<IdType = Static>> FusedIterator for RangeIter<E> {}

// #[cfg(feature = "rayon")]
// impl<E: Entity<IdType = Static> + Send> rayon::iter::ParallelIterator for RangeIter<E> {
//     type Item = Id<E>;
//
//     fn drive_unindexed<C>(self, consumer: C) -> C::Result
//     where
//         C: UnindexedConsumer<Self::Item>,
//     {
//         consumer.into_folder().consume_iter(self).complete()
//     }
// }
//
// #[cfg(feature = "rayon")]
// impl<E: Entity<IdType = Static> + Send> rayon::iter::IndexedParallelIterator for RangeIter<E> {
//     fn len(&self) -> usize {
//         self.range.len()
//     }
//
//     fn drive<C: rayon::iter::Consumer<Self::Item>>(self, consumer: C) -> C::Result {
//         consumer.into_folder().consume_iter(self).complete()
//     }
//
//     fn with_producer<CB: rayon::iter::ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
//         callback.callback(self)
//     }
// }
//
// #[cfg(feature = "rayon")]
// impl<E: Entity<IdType = Static> + Send> Producer for RangeIter<E> {
//     type Item = Id<E>;
//     type IntoIter = Self;
//
//     fn into_iter(self) -> Self::IntoIter {
//         self
//     }
//
//     fn split_at(self, index: usize) -> (Self, Self) {
//         let std::ops::Range { start, end } = self.range;
//         let mid = start + index as u32;
//         (RangeIter::new(start..mid), RangeIter::new(mid..end))
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::Gen;
    use crate::tests::{Dyn, Stat};
    use std::hash::{Hash, Hasher};

    impl<E: Entity<IdType = Dynamic>> Id<E> {
        pub(crate) fn next(self) -> Self {
            Self {
                index: self.index,
                gen: self.gen.next(),
                marker: Default::default(),
            }
        }
    }

    fn id_0_0() -> Id<Dyn> {
        Id::new(0, Gen::MIN)
    }

    fn id_0_1() -> Id<Dyn> {
        id_0_0().next()
    }

    fn id_1_0() -> Id<Dyn> {
        Id::new(1, Gen::MIN)
    }

    fn id_1_1() -> Id<Dyn> {
        id_1_0().next()
    }

    #[test]
    fn id_sizes() {
        use std::mem::size_of;
        assert_eq!(4, size_of::<Id<Stat>>());
        assert_eq!(4, size_of::<Option<Id<Stat>>>());
        assert_eq!(8, size_of::<Id<Dyn>>());
        assert_eq!(8, size_of::<Option<Id<Dyn>>>());
    }

    #[test]
    fn cmp() {
        let id0 = Id::<Dyn>::new(0, Gen::MIN);
        let id1 = Id::<Dyn>::new(1, Gen::MIN);

        assert!(NonMaxU32::new(0) > NonMaxU32::new(1));
        assert!(id0 < id1);
    }

    #[test]
    fn id_eq() {
        assert_eq!(id_0_0(), id_0_0());
        assert_ne!(id_0_0(), id_0_1());
        assert_ne!(id_0_0(), id_1_0());
    }

    #[test]
    fn id_clone() {
        assert_eq!(id_0_0(), id_0_0().clone());
        assert_ne!(id_0_0(), id_0_1().clone());
        assert_ne!(id_0_0(), id_1_0().clone());
    }

    #[test]
    fn id_partial_ord() {
        assert_eq!(Some(Ordering::Equal), id_0_0().partial_cmp(&id_0_0()));
        assert_eq!(Some(Ordering::Less), id_0_0().partial_cmp(&id_0_1()));
        assert_eq!(Some(Ordering::Less), id_0_0().partial_cmp(&id_1_0()));
        assert_eq!(Some(Ordering::Greater), id_1_1().partial_cmp(&id_0_0()));
        assert_eq!(Some(Ordering::Greater), id_1_1().partial_cmp(&id_1_0()));
    }

    #[test]
    fn id_ord() {
        assert_eq!(Ordering::Equal, id_0_0().cmp(&id_0_0()));
        assert_eq!(Ordering::Less, id_0_0().cmp(&id_0_1()));
        assert_eq!(Ordering::Less, id_0_0().cmp(&id_1_0()));
        assert_eq!(Ordering::Greater, id_1_1().cmp(&id_0_0()));
        assert_eq!(Ordering::Greater, id_1_1().cmp(&id_1_0()));
    }

    #[test]
    fn id_hash() {
        use fxhash::FxHasher;

        let default = FxHasher::default().finish();

        let get_hash = |id: Id<Dyn>| {
            let mut hasher = FxHasher::default();
            id.hash(&mut hasher);
            hasher.finish()
        };

        let hash_0_0 = get_hash(id_0_0());
        let hash_0_1 = get_hash(id_0_1());
        let hash_1_0 = get_hash(id_1_0());

        assert_ne!(hash_0_0, default);
        assert_ne!(hash_0_0, hash_0_1);
        assert_ne!(hash_0_0, hash_1_0);
    }

    #[test]
    fn id_index() {
        assert_eq!(0, id_0_0().index());
        assert_eq!(0, id_0_1().index());
        assert_eq!(1, id_1_0().index());
        assert_eq!(1, id_1_1().index());
    }

    #[test]
    fn id_partial_eq_valid_id() {
        use crate::Valid;

        let id0 = Id::<Dyn>::new(0, Gen::MIN);
        let valid0 = Valid::new(id0);

        let id1 = Id::<Dyn>::new(1, Gen::MIN);
        let valid1 = Valid::new(id1);

        assert_eq!(id0, valid0);
        assert_eq!(valid0, id0);
        assert_ne!(id0, valid1);
        assert_ne!(valid0, id1);
    }

    #[test]
    fn id_range_eq() {
        let range = IdRange::<Stat>::new(1, 2);
        let lower = IdRange::<Stat>::new(0, 2);
        let higher = IdRange::<Stat>::new(1, 3);

        assert_eq!(range, range);
        assert_ne!(range, lower);
        assert_ne!(range, higher);
    }

    #[test]
    fn id_range_clone() {
        let range = IdRange::<Stat>::new(1, 2);
        let lower = IdRange::<Stat>::new(0, 2);
        let higher = IdRange::<Stat>::new(1, 3);

        assert_eq!(range, range.clone());
        assert_ne!(range, lower.clone());
        assert_ne!(range, higher.clone());
    }

    #[test]
    fn id_range_is_empty() {
        assert!(IdRange::<Stat>::new(0, 0).is_empty());
        assert!(!IdRange::<Stat>::new(0, 1).is_empty());
    }

    #[test]
    fn id_range_len() {
        assert_eq!(0, IdRange::<Stat>::new(0, 0).len());
        assert_eq!(1, IdRange::<Stat>::new(0, 1).len());
    }

    #[test]
    fn id_range_contains() {
        let id0 = Id::<Stat>::new(0, ());
        assert!(!IdRange::<Stat>::new(0, 0).contains(id0));
        assert!(IdRange::<Stat>::new(0, 1).contains(id0));
    }

    #[test]
    fn id_range_from() {
        let id = Id::<Stat>::new(2, ());
        let range = IdRange::<Stat>::new(2, 3);

        assert_eq!(IdRange::from(id), range);
    }

    #[test]
    fn range_iter() {
        let mut iter = RangeIter::<Stat>::new(0..1);

        assert_eq!(Some(Id::new(0, ())), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn range_iter_back() {
        let mut iter = RangeIter::<Stat>::new(0..2);

        assert_eq!(Some(Id::new(1, ())), iter.next_back());
        assert_eq!(Some(Id::new(0, ())), iter.next_back());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn range_size_hint() {
        let iter = RangeIter::<Stat>::new(0..1);
        assert_eq!(iter.size_hint(), (0..1).size_hint());
    }

    #[test]
    fn send_id() {
        fn send<E: Entity>(id: Id<E>) {
            std::thread::spawn(move || dbg!(id));
        }

        let id = Id::<Stat>::new(1, ());
        send(id);
    }
}
