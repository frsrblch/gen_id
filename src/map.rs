use crate::allocator::KilledIds;
use crate::gen::AllocGen;
use crate::valid::Validator;
use crate::{Dynamic, Entity, Id, Valid, ValidId};
use ref_cast::RefCast;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawIdMap<E: Entity, T> {
    map: fxhash::FxHashMap<Id<E>, T>,
    gen: AllocGen<E>,
}

impl<E: Entity, T> Default for RawIdMap<E, T> {
    #[inline]
    fn default() -> Self {
        Self {
            map: Default::default(),
            gen: Default::default(),
        }
    }
}

impl<E: Entity, T: Clone> Clone for RawIdMap<E, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            gen: self.gen.clone(),
        }
    }
}

impl<E: Entity, T> RawIdMap<E, T> {
    #[inline]
    pub fn entry(&mut self, id: Id<E>) -> std::collections::hash_map::Entry<Id<E>, T> {
        self.map.entry(id)
    }

    #[inline]
    pub fn insert(&mut self, id: Id<E>, value: T) -> Option<T> {
        self.map.insert(id, value)
    }

    #[inline]
    pub fn remove(&mut self, id: &Id<E>) -> Option<T> {
        self.map.remove(id)
    }

    #[inline]
    pub fn get(&self, id: Id<E>) -> Option<&T> {
        self.map.get(&id)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id<E>) -> Option<&mut T> {
        self.map.get_mut(&id)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&Id<E>, &T)> + '_ {
        self.map.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Id<E>, &mut T)> + '_ {
        self.map.iter_mut()
    }

    #[inline]
    pub fn id_map(&self) -> &IdMap<E, T> {
        IdMap::ref_cast(self)
    }
}

impl<E: Entity<IdType = Dynamic>, T> RawIdMap<E, T> {
    #[inline]
    pub fn kill(&mut self, id: Id<E>) -> Option<T> {
        self.gen.increment(id);
        self.remove(&id)
    }

    #[inline]
    pub fn kill_many(&mut self, killed: &KilledIds<E>) {
        assert_eq!(&self.gen, killed.before());
        for id in killed.ids() {
            self.kill(*id.value);
        }
        assert_eq!(&self.gen, killed.after());
    }
}

impl<E: Entity, T> std::ops::Index<Id<E>> for RawIdMap<E, T> {
    type Output = T;
    #[inline]
    fn index(&self, index: Id<E>) -> &Self::Output {
        self.map.index(&index)
    }
}

impl<E: Entity, T> std::ops::Index<&Id<E>> for RawIdMap<E, T> {
    type Output = T;
    #[inline]
    fn index(&self, index: &Id<E>) -> &Self::Output {
        self.map.index(index)
    }
}

#[repr(transparent)]
#[derive(Debug, RefCast)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdMap<E: Entity, T> {
    map: RawIdMap<E, T>,
}

#[cfg(feature = "bevy")]
impl<E, T> bevy_ecs::prelude::Resource for IdMap<E, T>
where
    E: Entity + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
}

impl<E: Entity, T> Default for IdMap<E, T> {
    #[inline]
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<E: Entity, T: Clone> Clone for IdMap<E, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl<E: Entity, T> IdMap<E, T> {
    #[inline]
    pub fn entry<V: ValidId<Entity = E>>(
        &mut self,
        id: V,
    ) -> std::collections::hash_map::Entry<Id<E>, T> {
        self.map.entry(id.id())
    }

    #[inline]
    pub fn insert<V: ValidId<Entity = E>>(&mut self, id: V, value: T) -> Option<T> {
        self.map.insert(id.id(), value)
    }

    #[inline]
    pub fn remove<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<T> {
        self.map.remove(&id.id())
    }

    #[inline]
    pub fn get<V: ValidId<Entity = E>>(&self, id: V) -> Option<&T> {
        self.map.get(id.id())
    }

    #[inline]
    pub fn get_mut<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<&mut T> {
        self.map.get_mut(id.id())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&Id<E>, &T)> + '_ {
        self.map.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Id<E>, &mut T)> + '_ {
        self.map.iter_mut()
    }
}

impl<E: Entity<IdType = Dynamic>, T> IdMap<E, T> {
    #[inline]
    pub fn validate<'v, V: Validator<'v, E>>(&self, v: V) -> &Valid<'v, Self> {
        assert_eq!(
            &self.map.gen,
            v.as_ref(),
            "collection is out of sync with the allocator: {}",
            std::any::type_name::<Self>()
        );

        let _ = v;
        Valid::new_ref(self)
    }

    #[inline]
    pub fn validate_mut<'v, V: Validator<'v, E>>(&mut self, v: V) -> &mut Valid<'v, Self> {
        assert_eq!(
            &self.map.gen,
            v.as_ref(),
            "collection is out of sync with the allocator: {}",
            std::any::type_name::<Self>()
        );

        let _ = v;
        Valid::new_mut(self)
    }
}

impl<E: Entity<IdType = Dynamic>, T> IdMap<E, T> {
    #[inline]
    pub fn kill<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<T> {
        self.map.kill(id.id())
    }

    #[inline]
    pub fn kill_many(&mut self, killed: &KilledIds<E>) {
        self.map.kill_many(killed);
    }
}

impl<E: Entity, T, V: ValidId<Entity = E>> std::ops::Index<V> for IdMap<E, T> {
    type Output = T;
    #[inline]
    fn index(&self, index: V) -> &Self::Output {
        self.map.index(index.id())
    }
}

impl<'v, E: Entity, T> Valid<'v, IdMap<E, T>> {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Valid<'v, &Id<E>>, &T)> + '_ {
        self.value.iter().map(|(k, v)| (Valid::new(k), v))
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Valid<'v, &Id<E>>, &mut T)> + '_ {
        self.value.iter_mut().map(|(k, v)| (Valid::new(k), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::Dyn, Allocator};

    #[test]
    #[should_panic]
    fn validate_when_out_of_sync() {
        let mut a = Allocator::<Dyn>::default();
        let mut map = IdMap::<Dyn, ()>::default();

        let id = a.create();
        map.insert(id, ());

        let id = id.value;
        a.kill(id);

        map.validate(&a);
    }

    #[test]
    #[should_panic]
    fn validate_mut_when_out_of_sync() {
        let mut a = Allocator::<Dyn>::default();
        let mut map = IdMap::<Dyn, ()>::default();

        let id = a.create();
        map.insert(id, ());

        let id = id.value;
        a.kill(id);

        map.validate_mut(&a);
    }

    #[test]
    fn validate_when_in_sync() {
        let mut a = Allocator::<Dyn>::default();
        let mut map = IdMap::<Dyn, ()>::default();

        let id = a.create();
        map.insert(id, ());

        map.kill(id);
        let id = id.value;
        a.kill(id);

        map.validate(&a);
    }

    #[test]
    fn validate_mut_when_in_sync() {
        let mut a = Allocator::<Dyn>::default();
        let mut map = IdMap::<Dyn, ()>::default();

        let id = a.create();
        map.insert(id, ());

        map.kill(id);
        let id = id.value;
        a.kill(id);

        map.validate_mut(&a);
    }
}
