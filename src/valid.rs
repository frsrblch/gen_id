use crate::id::Id;
use crate::{Entity, Static};
use iter_context::ContextualIterator;
use ref_cast::RefCast;
use std::marker::PhantomData;

pub trait ValidId: Copy {
    type Entity: Entity;
    fn id(self) -> Id<Self::Entity>;
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, RefCast)]
pub struct Valid<'v, T> {
    pub value: T,
    marker: PhantomData<&'v ()>,
}

impl<T: PartialEq> PartialEq<T> for Valid<'_, T> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.value.eq(other)
    }
}

impl<'v, T> Valid<'v, T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            value,
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn new_ref(value: &T) -> &Self {
        Valid::ref_cast(value)
    }

    #[inline]
    pub fn new_mut(value: &mut T) -> &mut Self {
        Valid::ref_cast_mut(value)
    }

    #[inline]
    pub fn as_ref(&self) -> Valid<'v, &T> {
        Valid::new(&self.value)
    }
}

impl<E: Entity<IdType = Static>> ValidId for Id<E> {
    type Entity = E;
    #[inline]
    fn id(self) -> Id<E> {
        self
    }
}

impl<E: Entity<IdType = Static>> ValidId for &Id<E> {
    type Entity = E;
    #[inline]
    fn id(self) -> Id<E> {
        *self
    }
}

impl<'v, E: Entity> ValidId for Valid<'v, Id<E>> {
    type Entity = E;
    #[inline]
    fn id(self) -> Id<E> {
        self.value
    }
}

impl<'v, E: Entity> ValidId for &Valid<'v, Id<E>> {
    type Entity = E;
    #[inline]
    fn id(self) -> Id<E> {
        self.value
    }
}

impl<'v, E: Entity> ValidId for Valid<'v, &Id<E>> {
    type Entity = E;
    #[inline]
    fn id(self) -> Id<E> {
        *self.value
    }
}

impl<'v, E: Entity> ValidId for &Valid<'v, &Id<E>> {
    type Entity = E;
    #[inline]
    fn id(self) -> Id<E> {
        *self.value
    }
}

impl<'v, I: IntoIterator> IntoIterator for Valid<'v, I> {
    type Item = Valid<'v, I::Item>;
    type IntoIter = ValidIter<'v, I::IntoIter>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        ValidIter {
            iter: self.value.into_iter(),
            marker: PhantomData,
        }
    }
}

impl<'v, T: ContextualIterator> ContextualIterator for Valid<'v, T> {
    type Context = T::Context;
}

pub struct ValidIter<'v, I> {
    iter: I,
    marker: PhantomData<&'v ()>,
}

impl<'v, I: Iterator> Iterator for ValidIter<'v, I> {
    type Item = Valid<'v, I::Item>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Valid::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_partial_eq() {
        let valid = Valid::new(1);

        assert!(valid.eq(&1));
        assert!(!valid.eq(&0));
    }

    #[test]
    fn valid_eq() {
        let valid1 = Valid::new(1);
        let valid2 = Valid::new(2);

        assert_eq!(valid1, valid1);
        assert_ne!(valid1, valid2);
    }

    #[test]
    fn valid_iter() {
        let values = vec![1, 2, 3];
        let valid = Valid::new(values);
        let mut valid_iter = valid.into_iter();

        assert_eq!(Some(Valid::new(1)), valid_iter.next());
        assert_eq!(Some(Valid::new(2)), valid_iter.next());
        assert_eq!(Some(Valid::new(3)), valid_iter.next());
        assert_eq!(None, valid_iter.next());
    }

    #[test]
    fn valid_iter_ref() {
        let values = vec![1, 2, 3];
        let valid = Valid::new(&values);
        let mut valid_iter = valid.into_iter();

        assert_eq!(Some(Valid::new(&1)), valid_iter.next());
        assert_eq!(Some(Valid::new(&2)), valid_iter.next());
        assert_eq!(Some(Valid::new(&3)), valid_iter.next());
        assert_eq!(None, valid_iter.next());
    }
}
