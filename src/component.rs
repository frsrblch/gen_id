use crate::{Entity, Id, IdRange, Static, ValidId};
use iter_context::ContextualIterator;
use ref_cast::RefCast;
use std::marker::PhantomData;
use std::ops::*;

#[derive(Debug)]
pub struct RawComponent<E, T> {
    pub(crate) values: Vec<T>,
    marker: PhantomData<E>,
}

impl<E, T> Default for RawComponent<E, T> {
    #[inline]
    fn default() -> Self {
        Self {
            values: Vec::default(),
            marker: PhantomData,
        }
    }
}

impl<E, T: Clone> Clone for RawComponent<E, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            marker: PhantomData,
        }
    }
}

impl<E, T: PartialEq> PartialEq for RawComponent<E, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.values.eq(&other.values)
    }
}

impl<E, T: Eq> Eq for RawComponent<E, T> {}

impl<E: Entity, T> RawComponent<E, T> {
    #[track_caller]
    #[inline]
    pub fn insert(&mut self, id: Id<E>, value: T) {
        self.insert_with(id, value, || panic!("invalid index"));
    }

    #[inline]
    pub fn insert_with<F: FnMut() -> T>(&mut self, id: Id<E>, value: T, fill: F) {
        let index = id.index();
        if let Some(current) = self.values.get_mut(index) {
            *current = value;
        } else {
            if self.len() != index {
                let extend_by = index - self.values.len();
                let iter = std::iter::repeat_with(fill).take(extend_by);
                self.values.extend(iter);
            }
            self.values.push(value);
        }
    }

    #[inline]
    pub fn get(&self, id: Id<E>) -> Option<&T> {
        self.values.get(id.index())
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id<E>) -> Option<&mut T> {
        self.values.get_mut(id.index())
    }
}

impl<E, T> RawComponent<E, T> {
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> iter_context::Iter<E, T> {
        iter_context::Iter::new(&self.values)
    }

    #[inline]
    pub fn iter_mut(&mut self) -> iter_context::IterMut<E, T> {
        iter_context::IterMut::new(&mut self.values)
    }

    #[inline]
    pub fn fill_with<F: FnMut() -> T>(&mut self, fill: F) {
        self.values.fill_with(fill);
    }
}

impl<E: Entity, T> Index<Id<E>> for RawComponent<E, T> {
    type Output = T;
    #[inline]
    fn index(&self, index: Id<E>) -> &Self::Output {
        self.values.index(index.index())
    }
}

impl<E: Entity, T> IndexMut<Id<E>> for RawComponent<E, T> {
    #[inline]
    fn index_mut(&mut self, index: Id<E>) -> &mut Self::Output {
        self.values.index_mut(index.index())
    }
}

impl<E, T> IntoIterator for RawComponent<E, T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a, E, T> IntoIterator for &'a RawComponent<E, T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl<'a, E, T> IntoIterator for &'a mut RawComponent<E, T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter_mut()
    }
}

impl<E, T> ContextualIterator for RawComponent<E, T> {
    type Context = E;
}

impl<'a, E, T> ContextualIterator for &'a RawComponent<E, T> {
    type Context = E;
}

impl<'a, E, T> ContextualIterator for &'a mut RawComponent<E, T> {
    type Context = E;
}

impl<E: Entity<IdType = Static>, T> Index<IdRange<E>> for RawComponent<E, T> {
    type Output = [T];

    #[inline]
    fn index(&self, index: IdRange<E>) -> &Self::Output {
        self.values.index(index.range_usize())
    }
}

#[cfg(feature = "rayon")]
impl<'a, E, T: Sync> rayon::prelude::IntoParallelRefIterator<'a> for &'a RawComponent<E, T> {
    type Iter = rayon::slice::Iter<'a, T>;
    type Item = &'a T;

    #[inline]
    fn par_iter(&'a self) -> Self::Iter {
        self.values.as_slice().par_iter()
    }
}

#[repr(transparent)]
#[derive(Debug, RefCast)]
pub struct Component<E, T> {
    values: RawComponent<E, T>,
}

impl<E, T: PartialEq> PartialEq for Component<E, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.values.eq(&other.values)
    }
}

impl<E, T: Eq> Eq for Component<E, T> {}

impl<E, T> Default for Component<E, T> {
    #[inline]
    fn default() -> Self {
        Self {
            values: RawComponent::default(),
        }
    }
}

impl<E, T: Clone> Clone for Component<E, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
}

impl<E: Entity, T> Component<E, T> {
    #[inline]
    pub fn insert<V: ValidId<Entity = E>>(&mut self, id: V, value: T) {
        self.values.insert(id.id(), value);
    }

    #[inline]
    pub fn insert_with<V: ValidId<Entity = E>, F: FnMut() -> T>(
        &mut self,
        id: V,
        value: T,
        fill: F,
    ) {
        self.values.insert_with(id.id(), value, fill);
    }

    #[inline]
    pub fn get<V: ValidId<Entity = E>>(&self, id: V) -> Option<&T> {
        self.values.get(id.id())
    }

    #[inline]
    pub fn get_mut<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<&mut T> {
        self.values.get_mut(id.id())
    }
}

impl<E, T> Component<E, T> {
    #[inline]
    pub fn iter(&self) -> iter_context::Iter<E, T> {
        self.values.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> iter_context::IterMut<E, T> {
        self.values.iter_mut()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[inline]
    pub fn fill_with<F: FnMut() -> T>(&mut self, fill: F) {
        self.values.fill_with(fill);
    }
}

impl<E: Entity, T, V: ValidId<Entity = E>> Index<V> for Component<E, T> {
    type Output = T;
    #[inline]
    fn index(&self, index: V) -> &Self::Output {
        self.values.index(index.id())
    }
}

impl<E: Entity, T, V: ValidId<Entity = E>> IndexMut<V> for Component<E, T> {
    #[inline]
    fn index_mut(&mut self, index: V) -> &mut Self::Output {
        self.values.index_mut(index.id())
    }
}

impl<E, T> IntoIterator for Component<E, T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a, E, T> IntoIterator for &'a Component<E, T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&self.values).into_iter()
    }
}

impl<'a, E, T> IntoIterator for &'a mut Component<E, T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.values).into_iter()
    }
}

impl<E, T> ContextualIterator for Component<E, T> {
    type Context = E;
}

impl<'a, E, T> ContextualIterator for &'a Component<E, T> {
    type Context = E;
}

impl<'a, E, T> ContextualIterator for &'a mut Component<E, T> {
    type Context = E;
}

impl<E: Entity<IdType = Static>, T> Index<IdRange<E>> for Component<E, T> {
    type Output = [T];

    #[inline]
    fn index(&self, index: IdRange<E>) -> &Self::Output {
        self.values.values.index(index.range_usize())
    }
}

#[cfg(feature = "rayon")]
impl<'a, E, T: Sync> rayon::prelude::IntoParallelRefIterator<'a> for &'a Component<E, T> {
    type Iter = rayon::slice::Iter<'a, T>;
    type Item = &'a T;

    #[inline]
    fn par_iter(&'a self) -> Self::Iter {
        self.values.values.as_slice().par_iter()
    }
}

macro_rules! impl_assign_op {
    ($t:ident, $f:ident) => {
        impl<C, T, M, MItem> std::ops::$t<M> for RawComponent<C, T>
        where
            M: ContextualIterator<Context = C> + IntoIterator<Item = MItem>,
            T: std::ops::$t<MItem>,
        {
            #[inline]
            fn $f(&mut self, rhs: M) {
                self.iter_mut()
                    .zip(rhs)
                    .for_each(|(value, item)| value.$f(item));
            }
        }

        impl<C, T, M, MItem> std::ops::$t<M> for Component<C, T>
        where
            M: ContextualIterator<Context = C> + IntoIterator<Item = MItem>,
            T: std::ops::$t<MItem>,
        {
            #[inline]
            fn $f(&mut self, rhs: M) {
                self.values.$f(rhs);
            }
        }
    };
}

impl_assign_op!(AddAssign, add_assign);
impl_assign_op!(SubAssign, sub_assign);
impl_assign_op!(MulAssign, mul_assign);
impl_assign_op!(DivAssign, div_assign);
impl_assign_op!(BitOrAssign, bitor_assign);
impl_assign_op!(BitAndAssign, bitand_assign);
impl_assign_op!(BitXorAssign, bitxor_assign);

pub trait Assign<Item>: ContextualIterator {
    fn assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = Item>;
}

impl<C, T> Assign<T> for RawComponent<C, T> {
    #[inline]
    fn assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = T>,
    {
        self.iter_mut()
            .zip(rhs)
            .for_each(|(value, item)| *value = item);
    }
}

impl<'a, C, T: Copy + 'a> Assign<&'a T> for RawComponent<C, T> {
    #[inline]
    fn assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = &'a T>,
    {
        self.iter_mut()
            .zip(rhs)
            .for_each(|(value, item)| *value = *item);
    }
}

impl<C, T> Assign<T> for Component<C, T> {
    #[inline]
    fn assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = T>,
    {
        self.iter_mut()
            .zip(rhs)
            .for_each(|(value, item)| *value = item);
    }
}

impl<'a, C, T: Copy + 'a> Assign<&'a T> for Component<C, T> {
    #[inline]
    fn assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = &'a T>,
    {
        self.iter_mut()
            .zip(rhs)
            .for_each(|(value, item)| *value = *item);
    }
}

pub trait TryAssign<Item>: ContextualIterator {
    fn try_assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = Item>;
}

impl<C, T: Copy> TryAssign<Option<T>> for RawComponent<C, T> {
    fn try_assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = Option<T>>,
    {
        self.iter_mut().zip(rhs).for_each(|(value, item)| {
            *value = if let Some(item) = item { item } else { *value };
        });
    }
}

impl<'a, C, T: Copy + 'a> TryAssign<Option<&'a T>> for RawComponent<C, T> {
    fn try_assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = Option<&'a T>>,
    {
        self.iter_mut().zip(rhs).for_each(|(value, item)| {
            *value = if let Some(item) = item { *item } else { *value };
        });
    }
}

impl<'a, C, T: Copy + 'a> TryAssign<&'a Option<T>> for RawComponent<C, T> {
    fn try_assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = &'a Option<T>>,
    {
        self.iter_mut().zip(rhs).for_each(|(value, item)| {
            *value = if let Some(item) = item { *item } else { *value };
        });
    }
}

impl<C, T: Copy> TryAssign<Option<T>> for Component<C, T> {
    fn try_assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = Option<T>>,
    {
        self.iter_mut().zip(rhs).for_each(|(value, item)| {
            *value = if let Some(item) = item { item } else { *value };
        });
    }
}

impl<'a, C, T: Copy + 'a> TryAssign<Option<&'a T>> for Component<C, T> {
    fn try_assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = Option<&'a T>>,
    {
        self.iter_mut().zip(rhs).for_each(|(value, item)| {
            *value = if let Some(item) = item { *item } else { *value };
        });
    }
}

impl<'a, C, T: Copy + 'a> TryAssign<&'a Option<T>> for Component<C, T> {
    fn try_assign<I>(&mut self, rhs: I)
    where
        I: ContextualIterator<Context = Self::Context> + IntoIterator<Item = &'a Option<T>>,
    {
        self.iter_mut().zip(rhs).for_each(|(value, item)| {
            *value = if let Some(item) = item { *item } else { *value };
        });
    }
}

impl<C, T: Copy> Component<C, T> {
    pub fn use_assign<F, M>(&mut self, m: M, mut f: F)
    where
        F: FnMut(T, M::Item) -> T,
        M: ContextualIterator<Context = C>,
    {
        self.iter_mut()
            .zip(m)
            .for_each(|(value, item)| *value = f(*value, item));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Stat;

    impl<E, T> From<Vec<T>> for RawComponent<E, T> {
        #[inline]
        fn from(values: Vec<T>) -> Self {
            Self {
                values,
                marker: PhantomData,
            }
        }
    }

    impl<E, T> From<Vec<T>> for Component<E, T> {
        #[inline]
        fn from(values: Vec<T>) -> Self {
            Self {
                values: values.into(),
            }
        }
    }

    #[test]
    fn raw_component_clone() {
        let comp = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);
        let clone = comp.clone();
        assert_eq!(comp, clone);
    }

    #[test]
    fn raw_component_partial_eq() {
        let comp = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);
        let other = RawComponent::<Stat, u32>::from(vec![1, 1, 1]);
        assert_eq!(comp, comp);
        assert_ne!(comp, other);
    }

    #[test]
    fn raw_component_insert_next() {
        let mut comp = RawComponent::<Stat, u32>::default();
        comp.insert(Id::new(0, ()), 1);
        assert_eq!(RawComponent::from(vec![1]), comp);
    }

    #[test]
    #[should_panic]
    fn raw_component_insert_skipped() {
        let mut comp = RawComponent::<Stat, u32>::default();
        comp.insert(Id::new(1, ()), 1);
    }

    #[test]
    fn raw_component_insert_with_skipped() {
        let mut comp = RawComponent::<Stat, u32>::default();
        comp.insert_with(Id::new(1, ()), 1, || 0);
        assert_eq!(RawComponent::from(vec![0, 1]), comp);
    }

    #[test]
    fn raw_component_len() {
        let mut comp = RawComponent::<Stat, u32>::default();
        assert_eq!(0, comp.len());
        comp.insert(Id::new(0, ()), 1);
        assert_eq!(1, comp.len());
    }

    #[test]
    fn raw_component_is_empty() {
        let mut comp = RawComponent::<Stat, u32>::default();
        assert!(comp.is_empty());
        comp.insert(Id::new(0, ()), 1);
        assert!(!comp.is_empty());
    }

    #[test]
    fn raw_component_get_none() {
        let comp = RawComponent::<Stat, u32>::default();
        let id0 = Id::new(0, ());
        assert_eq!(None, comp.get(id0));
    }

    #[test]
    fn raw_component_get() {
        let comp = RawComponent::<Stat, u32>::from(vec![1]);
        let id0 = Id::new(0, ());
        assert_eq!(Some(&1), comp.get(id0));
    }

    #[test]
    fn raw_component_get_mut_none() {
        let mut comp = RawComponent::<Stat, u32>::default();
        let id0 = Id::new(0, ());
        assert_eq!(None, comp.get_mut(id0));
    }

    #[test]
    fn raw_component_get_mut() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![1]);
        let id0 = Id::new(0, ());
        assert_eq!(Some(&mut 1), comp.get_mut(id0));
    }

    #[test]
    fn raw_component_fill_with() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);
        comp.fill_with(|| 0);
        assert_eq!(RawComponent::from(vec![0, 0, 0]), comp);
    }

    #[test]
    fn raw_component_index() {
        let comp = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);
        let id1 = Id::new(1, ());
        assert_eq!(2, comp[id1]);
    }

    #[test]
    fn raw_component_index_mut() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);
        let id1 = Id::new(1, ());
        comp[id1] = 7;
        assert_eq!(7, comp[id1]);
    }

    #[test]
    fn component_clone() {
        let comp = Component::<Stat, u32>::from(vec![1, 2, 3]);
        let clone = comp.clone();
        assert_eq!(comp, clone);
    }

    #[test]
    fn component_partial_eq() {
        let comp = Component::<Stat, u32>::from(vec![1, 2, 3]);
        let other = Component::<Stat, u32>::from(vec![1, 1, 1]);
        assert_eq!(comp, comp);
        assert_ne!(comp, other);
    }

    #[test]
    fn component_insert_next() {
        let mut comp = Component::<Stat, u32>::default();
        comp.insert(Id::new(0, ()), 1);
        assert_eq!(Component::from(vec![1]), comp);
    }

    #[test]
    #[should_panic]
    fn component_insert_skipped() {
        let mut comp = Component::<Stat, u32>::default();
        comp.insert(Id::new(1, ()), 1);
    }

    #[test]
    fn component_insert_with_skipped() {
        let mut comp = Component::<Stat, u32>::default();
        comp.insert_with(Id::new(1, ()), 1, || 0);
        assert_eq!(Component::from(vec![0, 1]), comp);
    }

    #[test]
    fn component_len() {
        let mut comp = Component::<Stat, u32>::default();
        assert_eq!(0, comp.len());
        comp.insert(Id::new(0, ()), 1);
        assert_eq!(1, comp.len());
    }

    #[test]
    fn component_is_empty() {
        let mut comp = Component::<Stat, u32>::default();
        assert!(comp.is_empty());
        comp.insert(Id::new(0, ()), 1);
        assert!(!comp.is_empty());
    }

    #[test]
    fn component_get_none() {
        let comp = Component::<Stat, u32>::default();
        let id0 = Id::new(0, ());
        assert_eq!(None, comp.get(id0));
    }

    #[test]
    fn component_get() {
        let comp = Component::<Stat, u32>::from(vec![1]);
        let id0 = Id::new(0, ());
        assert_eq!(Some(&1), comp.get(id0));
    }

    #[test]
    fn component_get_mut_none() {
        let mut comp = Component::<Stat, u32>::default();
        let id0 = Id::new(0, ());
        assert_eq!(None, comp.get_mut(id0));
    }

    #[test]
    fn component_get_mut() {
        let mut comp = Component::<Stat, u32>::from(vec![1]);
        let id0 = Id::new(0, ());
        assert_eq!(Some(&mut 1), comp.get_mut(id0));
    }

    #[test]
    fn component_fill_with() {
        let mut comp = Component::<Stat, u32>::from(vec![1, 2, 3]);
        comp.fill_with(|| 0);
        assert_eq!(Component::from(vec![0, 0, 0]), comp);
    }

    #[test]
    fn component_index() {
        let comp = Component::<Stat, u32>::from(vec![1, 2, 3]);
        let id1 = Id::new(1, ());
        assert_eq!(2, comp[id1]);
    }

    #[test]
    fn component_index_mut() {
        let mut comp = Component::<Stat, u32>::from(vec![1, 2, 3]);
        let id1 = Id::new(1, ());
        comp[id1] = 7;
        assert_eq!(7, comp[id1]);
    }

    #[test]
    #[cfg(feature = "rayon")]
    fn raw_component_par_iter() {
        use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
        let comp = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);
        assert_eq!(3, (&comp).par_iter().count());
    }

    #[test]
    #[cfg(feature = "rayon")]
    fn component_par_iter() {
        use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
        let comp = Component::<Stat, u32>::from(vec![1, 2, 3]);
        assert_eq!(3, (&comp).par_iter().count());
    }

    #[test]
    fn raw_component_assign() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![0; 3]);
        let m = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);

        comp.assign(m.clone());

        assert_eq!(comp, m);
    }

    #[test]
    fn component_assign() {
        let mut comp = Component::<Stat, u32>::from(vec![0; 3]);
        let m = Component::<Stat, u32>::from(vec![1, 2, 3]);

        comp.assign(m.clone());

        assert_eq!(comp, m);
    }

    #[test]
    fn raw_component_assign_ref() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![0; 3]);
        let m = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);

        comp.assign(&m);

        assert_eq!(comp, m);
    }

    #[test]
    fn component_assign_ref() {
        let mut comp = Component::<Stat, u32>::from(vec![0; 3]);
        let m = Component::<Stat, u32>::from(vec![1, 2, 3]);

        comp.assign(&m);

        assert_eq!(comp, m);
    }

    #[test]
    fn raw_component_try_assign() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![0; 3]);
        let m = RawComponent::<Stat, Option<u32>>::from(vec![None, Some(2), None]);

        comp.try_assign(m);

        assert_eq!(comp, vec![0, 2, 0].into());
    }

    #[test]
    fn component_try_assign() {
        let mut comp = Component::<Stat, u32>::from(vec![0; 3]);
        let m = Component::<Stat, Option<u32>>::from(vec![None, Some(2), None]);

        comp.try_assign(m);

        assert_eq!(comp, vec![0, 2, 0].into());
    }

    #[test]
    fn raw_component_try_assign_ref_opt() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![0; 3]);
        let m = RawComponent::<Stat, Option<u32>>::from(vec![None, Some(2), None]);

        comp.try_assign(&m);

        assert_eq!(comp, vec![0, 2, 0].into());
    }

    #[test]
    fn component_try_assign_ref_opt() {
        let mut comp = Component::<Stat, u32>::from(vec![0; 3]);
        let m = Component::<Stat, Option<u32>>::from(vec![None, Some(2), None]);

        comp.try_assign(&m);

        assert_eq!(comp, vec![0, 2, 0].into());
    }

    #[test]
    fn raw_component_try_assign_opt_ref() {
        let mut comp = RawComponent::<Stat, u32>::from(vec![0; 3]);
        let m = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);

        comp.try_assign(m.iter().map(|v| v.rem(2).eq(&0).then_some(v)));

        assert_eq!(comp, vec![0, 2, 0].into());
    }

    #[test]
    fn component_try_assign_opt_ref() {
        let mut comp = Component::<Stat, u32>::from(vec![0; 3]);
        let m = Component::<Stat, u32>::from(vec![1, 2, 3]);

        comp.try_assign(m.iter().map(|v| (*v % 2 == 0).then_some(v)));

        assert_eq!(comp, vec![0, 2, 0].into());
    }

    #[test]
    fn raw_component_index_id_range() {
        let comp = RawComponent::<Stat, u32>::from(vec![1, 2, 3]);
        let range = IdRange::<Stat>::new(1, 3);
        assert_eq!(&[2, 3], &comp[range]);
    }

    #[test]
    fn component_index_id_range() {
        let comp = Component::<Stat, u32>::from(vec![1, 2, 3]);
        let range = IdRange::<Stat>::new(1, 3);
        assert_eq!(&[2, 3], &comp[range]);
    }

    #[test]
    fn component_use_assign() {
        let mut comp = Component::<Stat, i32>::from(vec![1, 2, 3]);
        let m = Component::<Stat, i32>::from(vec![2, 3, 5]);

        comp.use_assign(m, |v, m| {
            let d = m - v;
            d * d
        });

        assert_eq!(vec![1, 1, 4], comp.values.values);
    }

    // TODO test std::ops::_Assign ops
}
