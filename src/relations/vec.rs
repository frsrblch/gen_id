use super::*;

#[derive(Debug, ForceClone)]
pub enum VecRelation<E: Entity> {
    ChildOf(Id<E>),
    ParentOf(Vec<Id<E>>),
}

impl<E: Entity> PartialEq for VecRelation<E> {
    fn eq(&self, other: &Self) -> bool {
        use VecRelation::*;
        match (self, other) {
            (ChildOf(lhs), ChildOf(rhs)) => lhs.eq(rhs),
            (ParentOf(lhs), ParentOf(rhs)) => lhs.eq(rhs),
            _ => false,
        }
    }
}

impl<E: Entity> Eq for VecRelation<E> {}

impl<E: Entity> VecRelation<E> {
    #[inline]
    pub fn parent() -> Self {
        Self::ParentOf(Vec::default())
    }

    #[inline]
    pub fn parent_of(&self) -> Option<&Vec<Id<E>>> {
        match self {
            Self::ParentOf(c) => Some(c),
            Self::ChildOf(_) => None,
        }
    }

    #[inline]
    pub fn child_of(&self) -> Option<Id<E>> {
        match self {
            Self::ChildOf(p) => Some(*p),
            Self::ParentOf(_) => None,
        }
    }

    #[inline]
    pub fn is_parent(&self) -> bool {
        matches!(self, Self::ParentOf(_))
    }

    #[inline]
    pub fn is_child(&self) -> bool {
        !self.is_parent()
    }
}

#[derive(Debug, ForceDefault, ForceClone)]
pub struct VecRelations<E: Entity> {
    values: RawComponent<E, VecRelation<E>>,
}

/// Requires fixed because unlinking is not implemented
impl<E: Entity> VecRelations<E> {
    #[inline]
    #[track_caller]
    fn insert_if_empty(&mut self, id: impl ValidId<Entity = E>, relation: VecRelation<E>) {
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
    pub fn insert_parent(&mut self, id: impl ValidId<Entity = E>) {
        self.insert_if_empty(id, VecRelation::parent());
    }

    #[inline]
    #[track_caller]
    pub fn insert_child<V0: ValidId<Entity = E>, V1: ValidId<Entity = E>>(
        &mut self,
        id: V0,
        parent: V1,
    ) {
        match &mut self.values[parent.id()] {
            VecRelation::ParentOf(children) => children.push(id.id()),
            _ => panic!("parent id is not a parent"),
        }

        let relation = VecRelation::ChildOf(parent.id());
        self.insert_if_empty(id, relation);
    }
}

impl<E: Entity, V: ValidId<Entity = E>> Index<V> for VecRelations<E> {
    type Output = VecRelation<E>;

    #[inline]
    #[track_caller]
    fn index(&self, index: V) -> &Self::Output {
        self.values.index(index.id())
    }
}

impl<'a, E: Entity> IntoIterator for &'a VecRelations<E> {
    type Item = &'a VecRelation<E>;
    type IntoIter = <&'a RawComponent<E, VecRelation<E>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a, E: Entity> ContextualIterator for &'a VecRelations<E> {
    type Context = E;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Entity, RangeAllocator, Static};

    #[derive(Debug)]
    struct Arena;

    impl Entity for Arena {
        type IdType = Static;
    }

    #[test]
    fn get_children_for_new_parent_returns_empty_vec() {
        let mut graph = VecRelations::<Arena>::default();
        let mut alloc = RangeAllocator::<Arena>::default();

        let parent = alloc.create();

        graph.insert_parent(parent);
        assert_eq!(graph[parent], VecRelation::parent());
    }

    #[test]
    fn link_child_to_parent() {
        let mut graph = VecRelations::<Arena>::default();
        let mut alloc = RangeAllocator::<Arena>::default();

        let id0 = alloc.create();
        let id1 = alloc.create();

        graph.insert_parent(id0);
        graph.insert_child(id1, id0);

        assert_eq!(graph[id0], VecRelation::ParentOf(vec![id1.id()]));
        assert_eq!(graph[id1], VecRelation::ChildOf(id0.id()));
    }

    #[test]
    #[should_panic]
    fn link_child_to_another_child() {
        let mut graph = VecRelations::<Arena>::default();
        let mut alloc = RangeAllocator::<Arena>::default();

        let id0 = alloc.create();
        let id1 = alloc.create();
        let id2 = alloc.create();

        graph.insert_parent(id0);
        graph.insert_child(id1, id0);
        graph.insert_child(id2, id1);
    }

    #[test]
    #[should_panic]
    fn insert_parent_overtop_of_another_link() {
        let mut graph = VecRelations::<Arena>::default();
        let mut alloc = RangeAllocator::<Arena>::default();

        let id0 = alloc.create();

        graph.insert_parent(id0);
        graph.insert_parent(id0);
    }

    #[test]
    #[should_panic]
    fn insert_child_overtop_of_another_parent() {
        let mut graph = VecRelations::<Arena>::default();
        let mut alloc = RangeAllocator::<Arena>::default();

        let id0 = alloc.create();
        let id1 = alloc.create();

        graph.insert_parent(id0);
        graph.insert_parent(id1);
        graph.insert_child(id1, id0);
    }
}
