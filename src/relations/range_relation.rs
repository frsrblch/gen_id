use crate::component::RawComponent;
use crate::{Entity, Id, IdRange, Static, ValidId};
use iter_context::ContextualIterator;
use std::ops::Index;

#[derive(Debug)]
pub enum RangeRelation<E: Entity> {
    ChildOf(Id<E>),
    ParentOf(IdRange<E>),
}

impl<E: Entity> Clone for RangeRelation<E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: Entity> Copy for RangeRelation<E> {}

impl<E: Entity> PartialEq for RangeRelation<E> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ChildOf(l0), Self::ChildOf(r0)) => l0 == r0,
            (Self::ParentOf(l0), Self::ParentOf(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<E: Entity> Eq for RangeRelation<E> {}

unsafe impl<E: Entity> Send for RangeRelation<E> {}
unsafe impl<E: Entity> Sync for RangeRelation<E> {}

impl<E: Entity> RangeRelation<E> {
    #[inline]
    pub fn parent_of(self) -> Option<IdRange<E>> {
        match self {
            RangeRelation::ParentOf(c) => Some(c),
            RangeRelation::ChildOf(_) => None,
        }
    }

    #[inline]
    pub fn child_of(self) -> Option<Id<E>> {
        match self {
            RangeRelation::ChildOf(p) => Some(p),
            RangeRelation::ParentOf(_) => None,
        }
    }

    #[inline]
    pub fn is_parent(&self) -> bool {
        matches!(self, Self::ParentOf(_))
    }

    #[inline]
    pub fn is_child(&self) -> bool {
        matches!(self, Self::ChildOf(_))
    }
}

#[derive(Debug)]
pub struct RangeRelations<E: Entity> {
    values: RawComponent<E, RangeRelation<E>>,
}

impl<E: Entity> Default for RangeRelations<E> {
    #[inline]
    fn default() -> Self {
        Self {
            values: Default::default(),
        }
    }
}

impl<E: Entity> Clone for RangeRelations<E> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.values.clone_from(&source.values);
    }
}

/// Requires fixed because unlinking is not implemented
impl<E: Entity<IdType = Static>> RangeRelations<E> {
    #[inline]
    #[track_caller]
    fn insert_if_empty(&mut self, id: impl ValidId<Entity = E>, relation: RangeRelation<E>) {
        match self.values.get(id.id()) {
            None => self.values.insert(id.id(), relation),
            Some(_existing) => panic!(
                "{}::insert_if_empty: cannot insert over existing relation",
                std::any::type_name::<Self>()
            ),
        }
    }

    #[inline]
    #[track_caller]
    pub fn link(&mut self, parent: Id<E>, children: IdRange<E>) {
        self.insert_if_empty(parent, RangeRelation::ParentOf(children));
        for child in children {
            self.insert_if_empty(child, RangeRelation::ChildOf(parent));
        }
    }

    #[inline]
    pub fn parents<'a, I: IntoIterator<Item = Id<E>> + 'a>(
        &'a self,
        iter: I,
    ) -> impl Iterator<Item = Id<E>> + 'a {
        iter.into_iter()
            .filter(move |id| matches!(self[id], RangeRelation::ParentOf(_)))
    }
}

impl<E: Entity, V: ValidId<Entity = E>> Index<V> for RangeRelations<E> {
    type Output = RangeRelation<E>;

    #[inline]
    #[track_caller]
    fn index(&self, index: V) -> &Self::Output {
        self.values.index(index.id())
    }
}

impl<'a, E: Entity> IntoIterator for &'a RangeRelations<E> {
    type Item = &'a RangeRelation<E>;
    type IntoIter = <&'a RawComponent<E, RangeRelation<E>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.values).into_iter()
    }
}

impl<'a, E: Entity> ContextualIterator for &'a RangeRelations<E> {
    type Context = E;
}

#[cfg(feature = "rayon")]
impl<'a, E: Entity + 'a> rayon::iter::IntoParallelRefIterator<'a> for &'a RangeRelations<E> {
    type Iter = rayon::slice::Iter<'a, RangeRelation<E>>;
    type Item = &'a RangeRelation<E>;

    fn par_iter(&'a self) -> Self::Iter {
        self.values.values.as_slice().par_iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::Stat;

    #[test]
    fn range_relation_parent_of() {
        let range = IdRange::<Stat>::new(0, 1);
        let relation = RangeRelation::ParentOf(range);
        assert!(relation.is_parent());
        assert!(!relation.is_child());
        assert_eq!(Some(range), relation.parent_of());
        assert!(relation.child_of().is_none());
    }

    #[test]
    fn range_relation_child_of() {
        let id = Id::<Stat>::new(0, ());
        let relation = RangeRelation::ChildOf(id);
        assert!(!relation.is_parent());
        assert!(relation.is_child());
        assert!(relation.parent_of().is_none());
        assert_eq!(Some(id), relation.child_of());
    }

    #[test]
    fn get_children_for_new_parent_returns_empty_vec() {
        let mut graph = RangeRelations::<Stat>::default();
        let parent = Id::<Stat>::new(0, ());

        graph.link(parent, IdRange::default());

        assert_eq!(graph[parent], RangeRelation::ParentOf(IdRange::default()));
    }

    #[test]
    fn link_child_to_parent() {
        let mut graph = RangeRelations::<Stat>::default();

        let id0 = Id::new(0, ());
        let id1 = Id::new(1, ());

        graph.link(id0, IdRange::from(id1));

        assert_eq!(graph[id0], RangeRelation::ParentOf(IdRange::from(id1.id())));
        assert_eq!(graph[id1], RangeRelation::ChildOf(id0.id()));
    }

    #[test]
    #[should_panic]
    fn link_child_to_another_child() {
        let mut graph = RangeRelations::<Stat>::default();

        let id0 = Id::<Stat>::new(0, ());
        let children = IdRange::new(1, 2);
        let id2 = Id::<Stat>::new(2, ());

        graph.link(id0, children);
        graph.link(id2, children);
    }

    #[test]
    #[should_panic]
    fn insert_parent_overtop_of_another_link() {
        let mut graph = RangeRelations::<Stat>::default();

        let id0 = Id::<Stat>::new(0, ());

        graph.link(id0, IdRange::default());
        graph.link(id0, IdRange::default());
    }

    #[test]
    #[should_panic]
    fn insert_child_overtop_of_another_parent() {
        let mut graph = RangeRelations::<Stat>::default();

        let id0 = Id::<Stat>::new(0, ());
        let id1 = Id::<Stat>::new(1, ());

        graph.link(id0, IdRange::default());
        graph.link(id1, IdRange::new(0, 1));
    }

    #[test]
    #[should_panic]
    fn link_after_skipping_first_id() {
        let mut graph = RangeRelations::<Stat>::default();

        graph.link(Id::<Stat>::new(1, ()), Id::<Stat>::new(2, ()).into());
    }
}
