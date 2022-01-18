use crate::{Entity, Id, ValidId};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct RawComponent<E, T> {
    values: Vec<T>,
    marker: PhantomData<*const E>,
}

impl<E, T> Default for RawComponent<E, T> {
    fn default() -> Self {
        Self {
            values: Vec::default(),
            marker: PhantomData,
        }
    }
}

impl<E, T: Clone> Clone for RawComponent<E, T> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            marker: PhantomData,
        }
    }
}

impl<E, T> From<Vec<T>> for RawComponent<E, T> {
    fn from(values: Vec<T>) -> Self {
        Self {
            values,
            marker: PhantomData,
        }
    }
}

impl<E: Entity, T> RawComponent<E, T> {
    pub fn insert(&mut self, id: Id<E>, value: T) {
        self.insert_with(id, value, || panic!("invalid index"));
    }

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

    pub fn get(&self, id: Id<E>) -> Option<&T> {
        self.values.get(id.index())
    }

    pub fn get_mut(&mut self, id: Id<E>) -> Option<&mut T> {
        self.values.get_mut(id.index())
    }

    pub fn iter(&self) -> iter_context::Iter<E, T> {
        iter_context::Iter::new(&self.values)
    }

    pub fn iter_mut(&mut self) -> iter_context::IterMut<E, T> {
        iter_context::IterMut::new(&mut self.values)
    }

    pub fn fill_with<F: FnMut() -> T>(&mut self, fill: F) {
        self.values.fill_with(fill);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn component(&self) -> &Component<E, T> {
        let ptr = self as *const RawComponent<E, T> as *const Component<E, T>;
        unsafe { &*ptr }
    }
}

impl<E: Entity, T> std::ops::Index<Id<E>> for RawComponent<E, T> {
    type Output = T;
    fn index(&self, index: Id<E>) -> &Self::Output {
        self.values.index(index.index())
    }
}

impl<E: Entity, T> std::ops::IndexMut<Id<E>> for RawComponent<E, T> {
    fn index_mut(&mut self, index: Id<E>) -> &mut Self::Output {
        self.values.index_mut(index.index())
    }
}

impl<'a, E: Entity, T> IntoIterator for &'a RawComponent<E, T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl<'a, E: Entity, T> IntoIterator for &'a mut RawComponent<E, T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter_mut()
    }
}

impl<'a, E: Entity, T> iter_context::ContextualIterator for &'a RawComponent<E, T> {
    type Context = E;
}

impl<'a, E: Entity, T> iter_context::ContextualIterator for &'a mut RawComponent<E, T> {
    type Context = E;
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Component<E, T> {
    values: RawComponent<E, T>,
}

impl<E, T> Default for Component<E, T> {
    fn default() -> Self {
        Self {
            values: RawComponent::default(),
        }
    }
}

impl<E, T: Clone> Clone for Component<E, T> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
}

impl<E: Entity, T> From<Vec<T>> for Component<E, T> {
    fn from(values: Vec<T>) -> Self {
        Self {
            values: RawComponent::from(values),
        }
    }
}

impl<E: Entity, T> Component<E, T> {
    pub fn insert<V: ValidId<Entity = E>>(&mut self, id: V, value: T) {
        self.values.insert(id.id(), value);
    }

    pub fn insert_with<V: ValidId<Entity = E>, F: FnMut() -> T>(
        &mut self,
        id: V,
        value: T,
        fill: F,
    ) {
        self.values.insert_with(id.id(), value, fill);
    }

    pub fn get<V: ValidId<Entity = E>>(&self, id: V) -> Option<&T> {
        self.values.get(id.id())
    }

    pub fn get_mut<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<&mut T> {
        self.values.get_mut(id.id())
    }

    pub fn iter(&self) -> iter_context::Iter<E, T> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> iter_context::IterMut<E, T> {
        self.values.iter_mut()
    }

    pub fn fill_with<F: FnMut() -> T>(&mut self, fill: F) {
        self.values.fill_with(fill);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<E: Entity, T, V: ValidId<Entity = E>> std::ops::Index<V> for Component<E, T> {
    type Output = T;
    fn index(&self, index: V) -> &Self::Output {
        self.values.index(index.id())
    }
}

impl<E: Entity, T, V: ValidId<Entity = E>> std::ops::IndexMut<V> for Component<E, T> {
    fn index_mut(&mut self, index: V) -> &mut Self::Output {
        self.values.index_mut(index.id())
    }
}

impl<'a, E: Entity, T> IntoIterator for &'a Component<E, T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.values).into_iter()
    }
}

impl<'a, E: Entity, T> IntoIterator for &'a mut Component<E, T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.values).into_iter()
    }
}

impl<'a, E: Entity, T> iter_context::ContextualIterator for &'a Component<E, T> {
    type Context = E;
}

impl<'a, E: Entity, T> iter_context::ContextualIterator for &'a mut Component<E, T> {
    type Context = E;
}
