use crate::component::RawComponent;
use crate::{AllocGen, Dynamic, Entity, Id, Killed, Static, Valid, ValidId, Validator};
use force_derive::{ForceClone, ForceDefault};
use std::marker::PhantomData;

pub struct Optional;
pub struct Required;

#[derive(Debug, ForceDefault, ForceClone)]
pub struct RawLinks<Source: Entity, Target: Entity, LinkType> {
    raw: RawComponent<Source, Option<Id<Target>>>,
    marker: PhantomData<*const LinkType>,

    #[cfg(debug_assertions)]
    source_gen: AllocGen<Source>,
    #[cfg(debug_assertions)]
    target_gen: AllocGen<Target>,
}

impl<Source: Entity, Target: Entity> RawLinks<Source, Target, Optional> {
    pub fn link(&mut self, source: Id<Source>, target: Id<Target>) {
        self.raw.insert_with(source, Some(target), Option::default);
    }

    pub fn unlink(&mut self, id: Id<Source>) {
        if let Some(value) = self.raw.get_mut(id) {
            *value = None;
        }
    }
}

impl<Source: Entity, Target: Entity> RawLinks<Source, Target, Required> {
    pub fn link(&mut self, source: Id<Source>, target: Id<Target>) {
        self.raw.insert(source, Some(target));
    }
}

impl<Source: Entity, Target: Entity<IdType = Dynamic>> RawLinks<Source, Target, Optional> {
    pub fn kill_target(&mut self, target: Id<Target>) {
        for id in self.raw.iter_mut() {
            *id = id.filter(|id| *id != target);
        }

        #[cfg(debug_assertions)]
        self.target_gen.increment(target);
    }

    pub fn kill_targets(&mut self, killed: &Killed<Target>) {
        #[cfg(debug_assertions)]
        assert!(self.target_gen == killed.before);

        for id in killed.ids() {
            self.kill_target(*id.value)
        }

        #[cfg(debug_assertions)]
        assert!(self.target_gen == killed.after);
    }
}

impl<Source: Entity<IdType = Dynamic>, Target: Entity> RawLinks<Source, Target, Optional> {
    pub fn kill_source(&mut self, id: Id<Source>) {
        self.unlink(id);

        #[cfg(debug_assertions)]
        self.source_gen.increment(id);
    }

    pub fn kill_sources(&mut self, killed: &Killed<Source>) {
        #[cfg(debug_assertions)]
        assert!(self.source_gen == killed.before);

        for id in killed.ids() {
            self.kill_source(*id.value);
        }

        #[cfg(debug_assertions)]
        assert!(self.source_gen == killed.after);
    }
}

#[repr(transparent)]
#[derive(Debug, Default, Clone)]
pub struct Links<Source: Entity, Target: Entity, LinkType> {
    raw: RawLinks<Source, Target, LinkType>,
}

impl<Source: Entity, Target: Entity> Links<Source, Target, Optional> {
    pub fn link<S: ValidId<Entity = Source>, T: ValidId<Entity = Target>>(
        &mut self,
        source: S,
        target: T,
    ) {
        self.raw.link(source.id(), target.id());
    }
}

impl<Source: Entity, Target: Entity> Links<Source, Target, Required> {
    pub fn link<S: ValidId<Entity = Source>, T: ValidId<Entity = Target>>(
        &mut self,
        source: S,
        target: T,
    ) {
        self.raw.link(source.id(), target.id());
    }
}

pub trait ValidateA<A: Entity>: Sized {
    fn validate<'v, VA: Validator<'v, A>>(&self, va: VA) -> &Valid<'v, Self>;
}

pub trait ValidateB<B: Entity>: Sized {
    fn validate<'v, VB: Validator<'v, B>>(&self, vb: VB) -> &Valid<'v, Self>;
}

pub trait ValidateAB<A: Entity, B: Entity>: Sized {
    fn validate<'v, VA: Validator<'v, A>, VB: Validator<'v, B>>(
        &self,
        va: VA,
        vb: VB,
    ) -> &Valid<'v, Self>;
}

impl<Source: Entity<IdType = Dynamic>, Target: Entity<IdType = Static>, LinkType> ValidateA<Source>
    for Links<Source, Target, LinkType>
{
    fn validate<'v, V: Validator<'v, Source>>(&self, validator: V) -> &Valid<'v, Self> {
        #[cfg(debug_assertions)]
        assert!(&self.raw.source_gen == validator.as_ref());

        Valid::new_ref(self)
    }
}

impl<Source: Entity<IdType = Static>, Target: Entity<IdType = Dynamic>, LinkType> ValidateB<Target>
    for Links<Source, Target, LinkType>
{
    fn validate<'v, VT: Validator<'v, Target>>(&self, validator: VT) -> &Valid<'v, Self> {
        #[cfg(debug_assertions)]
        assert!(&self.raw.target_gen == validator.as_ref());

        Valid::new_ref(self)
    }
}

impl<Source: Entity<IdType = Dynamic>, Target: Entity<IdType = Dynamic>, LinkType>
    ValidateAB<Source, Target> for Links<Source, Target, LinkType>
{
    fn validate<'v, VS: Validator<'v, Source>, VT: Validator<'v, Target>>(
        &self,
        va: VS,
        vb: VT,
    ) -> &Valid<'v, Self> {
        #[cfg(debug_assertions)]
        {
            assert!(&self.raw.source_gen == va.as_ref());
            assert!(&self.raw.target_gen == vb.as_ref());
        }

        Valid::new_ref(self)
    }
}
