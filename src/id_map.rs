#[cfg(debug_assertions)]
use crate::AllocGen;
use crate::{Entity, Id, Killed, Valid, ValidId, Validator};
use force_derive::*;

#[derive(Debug, ForceDefault)]
pub struct RawIdMap<E: Entity, T> {
    map: fxhash::FxHashMap<Id<E>, T>,

    #[cfg(debug_assertions)]
    gen: AllocGen<E>,
}

impl<E: Entity, T: Clone> Clone for RawIdMap<E, T> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            #[cfg(debug_assertions)]
            gen: self.gen.clone(),
        }
    }
}

impl<E: Entity, T> RawIdMap<E, T> {
    pub fn entry(&mut self, id: Id<E>) -> std::collections::hash_map::Entry<Id<E>, T> {
        self.map.entry(id)
    }

    pub fn insert(&mut self, id: Id<E>, value: T) -> Option<T> {
        self.map.insert(id, value)
    }

    pub fn remove(&mut self, id: &Id<E>) -> Option<T> {
        self.map.remove(id)
    }

    pub fn kill(&mut self, id: Id<E>) -> Option<T> {
        #[cfg(debug_assertions)]
        self.gen.increment(id);

        self.remove(&id)
    }

    pub fn kill_many(&mut self, killed: &Killed<E>) {
        #[cfg(debug_assertions)]
        assert!(self.gen == killed.before);

        for id in killed.ids() {
            self.kill(*id.value);
        }

        #[cfg(debug_assertions)]
        assert!(self.gen == killed.after);
    }

    pub fn get(&self, id: Id<E>) -> Option<&T> {
        self.map.get(&id)
    }

    pub fn get_mut(&mut self, id: Id<E>) -> Option<&mut T> {
        self.map.get_mut(&id)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn id_map(&self) -> &IdMap<E, T> {
        let ptr = self as *const RawIdMap<E, T> as *const IdMap<E, T>;
        unsafe { &*ptr }
    }
}

impl<E: Entity, T> std::ops::Index<Id<E>> for RawIdMap<E, T> {
    type Output = T;

    fn index(&self, index: Id<E>) -> &Self::Output {
        self.map.index(&index)
    }
}

impl<E: Entity, T> std::ops::Index<&Id<E>> for RawIdMap<E, T> {
    type Output = T;

    fn index(&self, index: &Id<E>) -> &Self::Output {
        self.map.index(index)
    }
}

#[repr(transparent)]
#[derive(Debug, ForceDefault)]
pub struct IdMap<E: Entity, T> {
    map: RawIdMap<E, T>,
}

impl<E: Entity, T: Clone> Clone for IdMap<E, T> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl<E: Entity, T> IdMap<E, T> {
    pub fn entry<V: ValidId<Entity = E>>(
        &mut self,
        id: V,
    ) -> std::collections::hash_map::Entry<Id<E>, T> {
        self.map.entry(id.id())
    }

    pub fn insert<V: ValidId<Entity = E>>(&mut self, id: V, value: T) -> Option<T> {
        self.map.insert(id.id(), value)
    }

    pub fn remove<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<T> {
        self.map.remove(&id.id())
    }

    pub fn kill<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<T> {
        self.map.kill(id.id())
    }

    pub fn kill_many(&mut self, killed: &Killed<E>) {
        self.map.kill_many(killed);
    }

    pub fn get<V: ValidId<Entity = E>>(&self, id: V) -> Option<&T> {
        self.map.get(id.id())
    }

    pub fn get_mut<V: ValidId<Entity = E>>(&mut self, id: V) -> Option<&mut T> {
        self.map.get_mut(id.id())
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id<E>, &T)> + '_ {
        self.map.map.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Id<E>, &mut T)> + '_ {
        self.map.map.iter_mut()
    }

    pub fn validate<'v, V: Validator<'v, E>>(&self, v: V) -> &Valid<'v, Self> {
        #[cfg(debug_assertions)]
        assert!(&self.map.gen == v.as_ref());

        let _ = v;
        Valid::new_ref(self)
    }

    pub fn validate_mut<'v, V: Validator<'v, E>>(&mut self, v: V) -> &mut Valid<'v, Self> {
        #[cfg(debug_assertions)]
        assert!(&self.map.gen == v.as_ref());

        let _ = v;
        Valid::new_mut(self)
    }
}

impl<E: Entity, T, V: ValidId<Entity = E>> std::ops::Index<V> for IdMap<E, T> {
    type Output = T;

    fn index(&self, index: V) -> &Self::Output {
        self.map.index(index.id())
    }
}

impl<'v, E: Entity, T> Valid<'v, IdMap<E, T>> {
    pub fn iter(&self) -> impl Iterator<Item = (Valid<'v, &Id<E>>, &T)> + '_ {
        self.value.map.map.iter().map(|(k, v)| (Valid::new(k), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Valid<'v, &Id<E>>, &mut T)> + '_ {
        self.value
            .map
            .map
            .iter_mut()
            .map(|(k, v)| (Valid::new(k), v))
    }
}
