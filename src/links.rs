#![allow(unused_macros)]

use crate::component::RawComponent;
use crate::id_map::RawIdMap;
#[cfg(debug_assertions)]
use crate::AllocGen;
use crate::{Entity, Fixed, Id, IdRange, Killed, Valid, ValidId, Validator};
use fxhash::FxHashSet;

#[derive(Debug)]
pub struct RawLinks<Parent: Entity, Child: Entity, Children> {
    #[cfg(debug_assertions)]
    pub parent_gen: AllocGen<Parent>,
    #[cfg(debug_assertions)]
    pub child_gen: AllocGen<Child>,

    pub parent: RawComponent<Child, Option<Id<Parent>>>,
    pub children: Children,
}

impl<Parent: Entity, Child: Entity, Children: Clone> Clone for RawLinks<Parent, Child, Children> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            parent_gen: self.parent_gen.clone(),
            #[cfg(debug_assertions)]
            child_gen: self.child_gen.clone(),

            parent: self.parent.clone(),
            children: self.children.clone(),
        }
    }
}

impl<Parent: Entity, Child: Entity, Children: Default> Default
    for RawLinks<Parent, Child, Children>
{
    fn default() -> Self {
        Self {
            #[cfg(debug_assertions)]
            parent_gen: Default::default(),
            #[cfg(debug_assertions)]
            child_gen: Default::default(),

            parent: Default::default(),
            children: Default::default(),
        }
    }
}

impl<Parent: Entity, Child: Entity>
    RawLinks<Parent, Child, RawComponent<Parent, FxHashSet<Id<Child>>>>
{
    pub fn link(&mut self, parent: Id<Parent>, child: Id<Child>) {
        self.unlink(child);
        self.parent.insert(child, Some(parent));
        match self.children.get_mut(parent) {
            Some(children) => {
                children.insert(child);
            }
            None => {
                let mut set = FxHashSet::default();
                set.insert(child);
                self.children.insert(parent, set);
            }
        }
    }

    pub fn unlink(&mut self, child: Id<Child>) {
        if let Some(parent) = self.parent.get_mut(child) {
            if let Some(parent) = parent.take() {
                if let Some(children) = self.children.get_mut(parent) {
                    children.remove(&child);

                    if children.len() * 4 < children.capacity() {
                        children.shrink_to_fit();
                    }
                }
            }
        }
    }

    pub fn get_children(&self, parent: Id<Parent>) -> Option<&FxHashSet<Id<Child>>> {
        self.children.get(parent)
    }

    pub fn get_parent(&self, child: Id<Child>) -> Option<&Id<Parent>> {
        self.parent.get(child)?.as_ref()
    }

    pub fn kill_parent(&mut self, parent: Id<Parent>) {
        #[cfg(debug_assertions)]
        self.parent_gen.increment(parent);

        self.unlink_parent(parent);
    }

    pub fn kill_child(&mut self, child: Id<Child>) {
        #[cfg(debug_assertions)]
        self.child_gen.increment(child);

        self.unlink(child);
    }

    pub fn unlink_parent(&mut self, parent: Id<Parent>) {
        if let Some(children) = self.children.get_mut(parent) {
            for child in children.drain() {
                self.parent.insert(child, None);
            }
        }
    }
}

impl<Parent: Entity, Child: Entity>
    RawLinks<Parent, Child, RawIdMap<Parent, FxHashSet<Id<Child>>>>
{
    pub fn link(&mut self, parent: Id<Parent>, child: Id<Child>) {
        self.unlink(child);
        self.parent.insert(child, Some(parent));
        self.children.entry(parent).or_default().insert(child);
    }

    pub fn unlink(&mut self, child: Id<Child>) {
        if let Some(parent) = self.parent.get_mut(child) {
            if let Some(parent) = parent.take() {
                if let Some(children) = self.children.get_mut(parent) {
                    children.remove(&child);

                    // shrink allocation if necessary
                    if children.is_empty() {
                        self.children.remove(&parent);
                    } else if children.len() * 4 < children.capacity() {
                        children.shrink_to_fit();
                    }
                }
            }
        }
    }

    pub fn get_children(&self, parent: Id<Parent>) -> Option<&FxHashSet<Id<Child>>> {
        self.children.get(parent)
    }

    pub fn get_parent(&self, child: Id<Child>) -> Option<&Id<Parent>> {
        self.parent.get(child)?.as_ref()
    }

    pub fn kill_parent(&mut self, parent: Id<Parent>) {
        #[cfg(debug_assertions)]
        self.parent_gen.increment(parent);

        self.unlink_parent(parent);
    }

    pub fn kill_parents(&mut self, killed: &Killed<Parent>) {
        #[cfg(debug_assertions)]
        assert!(killed.before() == &self.parent_gen);

        for p in killed.ids() {
            self.kill_parent(p.id());
        }

        #[cfg(debug_assertions)]
        assert!(killed.after() == &self.parent_gen);
    }

    pub fn kill_child(&mut self, child: Id<Child>) {
        #[cfg(debug_assertions)]
        self.child_gen.increment(child);

        self.unlink(child);
    }

    pub fn kill_children(&mut self, killed: &Killed<Child>) {
        #[cfg(debug_assertions)]
        assert!(killed.before() == &self.child_gen);

        for c in killed.ids() {
            self.kill_child(c.id());
        }

        #[cfg(debug_assertions)]
        assert!(killed.after() == &self.child_gen);
    }

    pub fn unlink_parent(&mut self, parent: Id<Parent>) {
        if let Some(children) = self.children.remove(&parent) {
            for child in children {
                self.parent.insert(child, None);
            }
        }
    }
}

impl<Parent: Entity<IdType = Fixed>, Child: Entity<IdType = Fixed>>
    RawLinks<Parent, Child, RawComponent<Parent, IdRange<Child>>>
{
    pub fn link(&mut self, parent: Id<Parent>, child: Id<Child>) {
        self.parent.insert(child, Some(parent));
        match self.children.get_mut(parent) {
            Some(children) => {
                children.append(child);
            }
            None => {
                self.children.insert(parent, IdRange::from(child));
            }
        }
    }

    pub fn get_children(&self, parent: Id<Parent>) -> Option<&IdRange<Child>> {
        self.children.get(parent)
    }

    pub fn get_parent(&self, child: Id<Child>) -> Option<&Id<Parent>> {
        self.parent.get(child)?.as_ref()
    }
}

impl<Parent: Entity<IdType = Fixed>, Child: Entity<IdType = Fixed>>
    RawLinks<Parent, Child, RawIdMap<Parent, IdRange<Child>>>
{
    pub fn link(&mut self, parent: Id<Parent>, child: Id<Child>) {
        self.parent.insert(child, Some(parent));
        match self.children.get_mut(parent) {
            Some(children) => {
                children.append(child);
            }
            None => {
                self.children.insert(parent, IdRange::from(child));
            }
        }
    }

    pub fn get_children(&self, parent: Id<Parent>) -> Option<&IdRange<Child>> {
        self.children.get(parent)
    }

    pub fn get_parent(&self, child: Id<Child>) -> Option<&Id<Parent>> {
        self.parent.get(child)?.as_ref()
    }
}

impl<Parent: Entity, Child: Entity, Children> RawLinks<Parent, Child, Children> {
    pub fn validate_both<'v, P: Validator<'v, Parent>, C: Validator<'v, Child>>(
        &self,
        p: P,
        c: C,
    ) -> &Valid<'v, Self> {
        #[cfg(debug_assertions)]
        {
            assert!(&self.parent_gen == p.as_ref());
            assert!(&self.child_gen == c.as_ref());
        }

        Valid::new_ref(self)
    }

    pub fn validate_parent<'v, P: Validator<'v, Parent>>(&self, p: P) -> &Valid<'v, Self> {
        #[cfg(debug_assertions)]
        {
            assert!(&self.parent_gen == p.as_ref());
        }

        Valid::new_ref(self)
    }

    pub fn validate_child<'v, C: Validator<'v, Child>>(&self, c: C) -> &Valid<'v, Self> {
        #[cfg(debug_assertions)]
        {
            assert!(&self.child_gen == c.as_ref());
        }

        Valid::new_ref(self)
    }
}

#[macro_export]
macro_rules! dense_range_link {
    ($ty:ident, $parent:ident, $child:ident) => {
        #[derive(Debug, Default, Clone)]
        pub struct $ty {
            links: $crate::links::RawLinks<
                $parent,
                $child,
                $crate::component::RawComponent<$parent, $crate::IdRange<$child>>,
            >,
        }

        impl $ty {
            pub fn link(&mut self, parent: $crate::Id<$parent>, child: $crate::Id<$child>) {
                self.links.link(parent, child);
            }

            pub fn get_children(
                &self,
                parent: $crate::Id<$parent>,
            ) -> Option<&$crate::IdRange<$child>> {
                self.links.get_children(parent)
            }

            pub fn get_parent(&self, child: $crate::Id<$child>) -> Option<&$crate::Id<$parent>> {
                self.links.get_parent(child)
            }
        }

        impl std::ops::Index<$crate::Id<$parent>> for $ty {
            type Output = $crate::IdRange<$child>;
            fn index(&self, index: $crate::Id<$parent>) -> &Self::Output {
                self.links.children.index(index)
            }
        }

        impl std::ops::Index<$crate::Id<$child>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: $crate::Id<$child>) -> &Self::Output {
                self.links.parent.index(index).as_ref().unwrap()
            }
        }
    };
}

#[macro_export]
macro_rules! sparse_range_link {
    ($ty:ident, $parent:ident, $child:ident) => {
        #[derive(Debug, Default, Clone)]
        pub struct $ty {
            links: $crate::links::RawLinks<
                $parent,
                $child,
                $crate::id_map::RawIdMap<$parent, $crate::IdRange<$child>>,
            >,
        }

        impl $ty {
            pub fn link(&mut self, parent: $crate::Id<$parent>, child: $crate::Id<$child>) {
                self.links.link(parent, child);
            }

            pub fn get_children(
                &self,
                parent: $crate::Id<$parent>,
            ) -> Option<&$crate::IdRange<$child>> {
                self.links.get_children(parent)
            }

            pub fn get_parent(&self, child: $crate::Id<$child>) -> Option<&$crate::Id<$parent>> {
                self.links.get_parent(child)
            }
        }

        impl std::ops::Index<$crate::Id<$parent>> for $ty {
            type Output = $crate::IdRange<$child>;
            fn index(&self, index: $crate::Id<$parent>) -> &Self::Output {
                self.links.children.index(index)
            }
        }

        impl std::ops::Index<$crate::Id<$child>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: $crate::Id<$child>) -> &Self::Output {
                self.links.parent.index(index).as_ref().unwrap()
            }
        }
    };
}

#[macro_export]
macro_rules! dense_required_link {
    ($ty:ident, $parent:ident: Fixed, $child:ident: Fixed) => {
        $crate::define_links_inner!($ty, $parent: Component, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Fixed, Required);
    };
    ($ty:ident, $parent:ident: Fixed, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: Component, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Required);

        impl $ty {
            $crate::kill_child!($child);

            $crate::validate!($parent: Fixed, $child: Dynamic);
        }
    };
}

#[macro_export]
macro_rules! dense_optional_link {
    ($ty:ident, $parent:ident: Fixed, $child:ident: Fixed) => {
        $crate::define_links_inner!($ty, $parent: Component, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Fixed, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);
        }
    };
    ($ty:ident, $parent:ident: Fixed, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: Component, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);

            $crate::kill_child!($child);

            $crate::validate!($parent: Fixed, $child: Dynamic);
        }
    };
    ($ty:ident, $parent:ident: Dynamic, $child:ident: Fixed) => {
        $crate::define_links_inner!($ty, $parent: Component, $child);

        $crate::index_parent!($ty, $parent: Dynamic, $child);
        $crate::index_child!($ty, $parent, $child: Fixed, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);

            $crate::kill_parent!($parent);

            $crate::validate!($parent: Dynamic, $child: Fixed);
        }
    };
    ($ty:ident, $parent:ident: Dynamic, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: Component, $child);

        $crate::index_parent!($ty, $parent: Dynamic, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);

            $crate::kill_parent!($parent);

            $crate::kill_child!($child);

            $crate::validate!($parent: Dynamic, $child: Dynamic);
        }
    };
}

#[macro_export]
macro_rules! sparse_required_link {
    ($ty:ident, $parent:ident: Fixed, $child:ident: Fixed) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Fixed, Required);
    };
    ($ty:ident, $parent:ident: Fixed, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Required);

        impl $ty {
            $crate::kill_child!($child);

            $crate::validate!($parent: Fixed, $child: Dynamic);
        }
    };
}

#[macro_export]
macro_rules! sparse_optional_link {
    ($ty:ident, $parent:ident: Fixed, $child:ident: Fixed) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Fixed, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);
        }
    };
    ($ty:ident, $parent:ident: Fixed, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);

            $crate::kill_child!($child);

            $crate::validate!($parent: Fixed, $child: Dynamic);
        }
    };
    ($ty:ident, $parent:ident: Dynamic, $child:ident: Fixed) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Dynamic, $child);
        $crate::index_child!($ty, $parent, $child: Fixed, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);

            $crate::kill_parent!($parent);

            $crate::validate!($parent: Dynamic, $child: Fixed);
        }
    };
    ($ty:ident, $parent:ident: Dynamic, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Dynamic, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);

            $crate::kill_parent!($parent);

            $crate::kill_child!($child);

            $crate::validate!($parent: Dynamic, $child: Dynamic);
        }
    };
}

#[macro_export]
macro_rules! define_links_inner {
    ($ty:ident, $parent:ident: Component, $child:ident) => {
        #[derive(Debug, Default, Clone)]
        pub struct $ty {
            links: $crate::links::RawLinks<
                $parent,
                $child,
                $crate::component::RawComponent<$parent, $crate::FxHashSet<$crate::Id<$child>>>,
            >,
        }

        impl $ty {
            pub fn link<
                P: $crate::ValidId<Entity = $parent>,
                C: $crate::ValidId<Entity = $child>,
            >(
                &mut self,
                parent: P,
                child: C,
            ) {
                self.links.link(parent.id(), child.id());
            }

            pub fn get_children<P: $crate::ValidId<Entity = $parent>>(
                &self,
                parent: P,
            ) -> std::option::Option<&$crate::FxHashSet<$crate::Id<$child>>> {
                self.links.get_children(parent.id())
            }

            pub fn get_parent<C: $crate::ValidId<Entity = $child>>(
                &self,
                child: C,
            ) -> std::option::Option<&$crate::Id<$parent>> {
                self.links.get_parent(child.id())
            }
        }
    };
    ($ty:ident, $parent:ident: IdMap, $child:ident) => {
        #[derive(Debug, Default)]
        pub struct $ty {
            links: $crate::links::RawLinks<
                $parent,
                $child,
                $crate::id_map::RawIdMap<$parent, $crate::FxHashSet<$crate::Id<$child>>>,
            >,
        }

        impl $ty {
            pub fn link<
                P: $crate::ValidId<Entity = $parent>,
                C: $crate::ValidId<Entity = $child>,
            >(
                &mut self,
                parent: P,
                child: C,
            ) {
                self.links.link(parent.id(), child.id());
            }

            pub fn get_children<P: $crate::ValidId<Entity = $parent>>(
                &self,
                parent: P,
            ) -> std::option::Option<&$crate::FxHashSet<$crate::Id<$child>>> {
                self.links.get_children(parent.id())
            }

            pub fn get_parent<C: $crate::ValidId<Entity = $child>>(
                &self,
                child: C,
            ) -> std::option::Option<&$crate::Id<$parent>> {
                self.links.get_parent(child.id())
            }
        }
    };
}

#[macro_export]
macro_rules! index_parent {
    ($ty:ident, $parent:ident: Fixed, $child:ident) => {
        impl std::ops::Index<$crate::Id<$parent>> for $ty {
            type Output = $crate::FxHashSet<$crate::Id<$child>>;
            fn index(&self, index: $crate::Id<$parent>) -> &Self::Output {
                self.links.children.index(index)
            }
        }

        impl std::ops::Index<&$crate::Id<$parent>> for $ty {
            type Output = $crate::FxHashSet<$crate::Id<$child>>;
            fn index(&self, index: &$crate::Id<$parent>) -> &Self::Output {
                self.links.children.index(*index)
            }
        }
    };
    ($ty:ident, $parent:ident: Dynamic, $child:ident) => {
        impl std::ops::Index<$crate::Valid<'_, $crate::Id<$parent>>> for $ty {
            type Output = $crate::FxHashSet<$crate::Id<$child>>;
            fn index(&self, index: $crate::Valid<$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index(index.id())
            }
        }

        impl std::ops::Index<$crate::Valid<'_, &$crate::Id<$parent>>> for $ty {
            type Output = $crate::FxHashSet<$crate::Id<$child>>;
            fn index(&self, index: $crate::Valid<&$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index(index.id())
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, $crate::Id<$parent>>> for $ty {
            type Output = $crate::FxHashSet<$crate::Id<$child>>;
            fn index(&self, index: &$crate::Valid<$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index(index.id())
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, &$crate::Id<$parent>>> for $ty {
            type Output = $crate::FxHashSet<$crate::Id<$child>>;
            fn index(&self, index: &$crate::Valid<&$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index(index.id())
            }
        }
    };
}

#[macro_export]
macro_rules! index_child {
    ($ty:ident, $parent:ident, $child:ident: Fixed, Optional) => {
        impl std::ops::Index<$crate::Id<$child>> for $ty {
            type Output = std::option::Option<$crate::Id<$parent>>;
            fn index(&self, index: $crate::Id<$child>) -> &Self::Output {
                self.links.parent.index(index)
            }
        }

        impl std::ops::Index<&$crate::Id<$child>> for $ty {
            type Output = std::option::Option<$crate::Id<$parent>>;
            fn index(&self, index: &$crate::Id<$child>) -> &Self::Output {
                self.links.parent.index(*index)
            }
        }
    };
    ($ty:ident, $parent:ident, $child:ident: Fixed, Required) => {
        impl std::ops::Index<$crate::Id<$child>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: $crate::Id<$child>) -> &Self::Output {
                self.links.parent.index(index).as_ref().unwrap()
            }
        }

        impl std::ops::Index<&$crate::Id<$child>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: &$crate::Id<$child>) -> &Self::Output {
                self.links.parent.index(*index).as_ref().unwrap()
            }
        }
    };
    ($ty:ident, $parent:ident, $child:ident: Dynamic, Optional) => {
        impl std::ops::Index<$crate::Valid<'_, $crate::Id<$child>>> for $ty {
            type Output = std::option::Option<$crate::Id<$parent>>;
            fn index(&self, index: $crate::Valid<$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(index.value)
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, $crate::Id<$child>>> for $ty {
            type Output = std::option::Option<$crate::Id<$parent>>;
            fn index(&self, index: &$crate::Valid<$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(index.value)
            }
        }

        impl std::ops::Index<$crate::Valid<'_, &$crate::Id<$child>>> for $ty {
            type Output = std::option::Option<$crate::Id<$parent>>;
            fn index(&self, index: $crate::Valid<&$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(*index.value)
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, &$crate::Id<$child>>> for $ty {
            type Output = std::option::Option<$crate::Id<$parent>>;
            fn index(&self, index: &$crate::Valid<&$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(*index.value)
            }
        }
    };
    ($ty:ident, $parent:ident, $child:ident: Dynamic, Required) => {
        impl std::ops::Index<$crate::Valid<'_, $crate::Id<$child>>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: $crate::Valid<$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(index.value).as_ref().unwrap()
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, $crate::Id<$child>>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: &$crate::Valid<$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(index.value).as_ref().unwrap()
            }
        }

        impl std::ops::Index<$crate::Valid<'_, &$crate::Id<$child>>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: $crate::Valid<&$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(*index.value).as_ref().unwrap()
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, &$crate::Id<$child>>> for $ty {
            type Output = $crate::Id<$parent>;
            fn index(&self, index: &$crate::Valid<&$crate::Id<$child>>) -> &Self::Output {
                self.links.parent.index(*index.value).as_ref().unwrap()
            }
        }
    };
}

#[macro_export]
macro_rules! unlink {
    ($parent:ident, $child:ident) => {
        pub fn unlink_parent<P: $crate::ValidId<Entity = $parent>>(&mut self, p: P) {
            self.links.unlink_parent(p.id());
        }

        pub fn unlink<C: $crate::ValidId<Entity = $child>>(&mut self, child: C) {
            self.links.unlink(child.id());
        }
    };
}

#[macro_export]
macro_rules! kill_parent {
    ($parent:ident) => {
        pub fn kill_parent<P: $crate::ValidId<Entity = $parent>>(&mut self, parent: P) {
            self.links.kill_parent(parent.id());
        }

        pub fn kill_parents(&mut self, killed: &$crate::Killed<$parent>) {
            #[cfg(debug_assertions)]
            assert!(killed.before() == &self.links.parent_gen);

            for p in killed.ids() {
                self.kill_parent(p);
            }

            #[cfg(debug_assertions)]
            assert!(killed.after() == &self.links.parent_gen);
        }
    };
}

#[macro_export]
macro_rules! kill_child {
    ($child:ident) => {
        pub fn kill_child<C: $crate::ValidId<Entity = $child>>(&mut self, child: C) {
            self.links.kill_child(child.id());
        }

        pub fn kill_children(&mut self, killed: &$crate::Killed<$child>) {
            #[cfg(debug_assertions)]
            assert!(killed.before() == &self.links.child_gen);

            for c in killed.ids() {
                self.kill_child(c);
            }

            #[cfg(debug_assertions)]
            assert!(killed.after() == &self.links.child_gen);
        }
    };
}

#[macro_export]
macro_rules! validate {
    ($parent:ident: Fixed, $child:ident: Fixed) => {};
    ($parent:ident: Fixed, $child:ident: Dynamic) => {
        #[allow(unused_variables)]
        pub fn validate<'v, V: $crate::Validator<'v, $child>>(
            &self,
            v: V,
        ) -> &$crate::Valid<'v, Self> {
            let _ = self.links.validate_child(v);
            Valid::new_ref(self)
        }
    };
    ($parent:ident: Dynamic, $child:ident: Fixed) => {
        #[allow(unused_variables)]
        pub fn validate<'v, V: $crate::Validator<'v, $parent>>(
            &self,
            v: V,
        ) -> &$crate::Valid<'v, Self> {
            let _ = self.links.validate_parent(v);
            Valid::new_ref(self)
        }
    };
    ($parent:ident: Dynamic, $child:ident: Dynamic) => {
        #[allow(unused_variables)]
        pub fn validate<'v, P: $crate::Validator<'v, $parent>, C: $crate::Validator<'v, $child>>(
            &self,
            p: P,
            c: C,
        ) -> &$crate::Valid<'v, Self> {
            let _ = self.links.validate_both(p, c);
            Valid::new_ref(self)
        }
    };
}

#[cfg(test)]
mod test {
    #![allow(dead_code)]

    use super::*;
    use crate::{Allocator, Dynamic, Entity, Fixed};

    #[derive(Debug)]
    pub struct FixA;

    impl Entity for FixA {
        type IdType = Fixed;
    }

    #[derive(Debug)]
    pub struct DynA;

    impl Entity for DynA {
        type IdType = Dynamic;
    }

    #[derive(Debug)]
    pub struct FixB;

    impl Entity for FixB {
        type IdType = Fixed;
    }

    #[derive(Debug)]
    pub struct DynB;

    impl Entity for DynB {
        type IdType = Dynamic;
    }

    dense_range_link!(FixFixRange, FixA, FixB);
    sparse_range_link!(FixFixRangeMap, FixA, FixB);

    dense_required_link!(FixFixReq, FixA: Fixed, FixB: Fixed);
    dense_required_link!(FixDynReq, FixA: Fixed, DynB: Dynamic);

    dense_optional_link!(DynDynOpt, DynA: Dynamic, DynB: Dynamic);
    dense_optional_link!(DynFixOpt, DynA: Dynamic, FixB: Fixed);
    dense_optional_link!(FixDynOpt, FixA: Fixed, DynB: Dynamic);
    dense_optional_link!(FixFixOpt, FixA: Fixed, FixB: Fixed);

    sparse_required_link!(FixFixReqMap, FixA: Fixed, FixB: Fixed);
    sparse_required_link!(FixDynReqMap, FixA: Fixed, DynB: Dynamic);

    sparse_optional_link!(DynDynOptMap, DynA: Dynamic, DynB: Dynamic);
    sparse_optional_link!(DynFixOptMap, DynA: Dynamic, FixB: Fixed);
    sparse_optional_link!(FixDynOptMap, FixA: Fixed, DynB: Dynamic);
    sparse_optional_link!(FixFixOptMap, FixA: Fixed, FixB: Fixed);

    #[test]
    fn test() {
        let mut links = DynDynOptMap::default();
        let mut dyn_a = Allocator::<DynA>::default();
        let mut dyn_b = Allocator::<DynB>::default();

        let id_a = dyn_a.create();
        let id_b = dyn_b.create();

        links.link(id_a, id_b);

        assert!(links[id_a].contains(&id_b.id()));
        assert_eq!(Some(id_a.id()), links[id_b]);

        links.unlink(id_b);

        assert!(links.get_children(id_a).is_none());
        assert_eq!(None, links[id_b]);

        let mut ids = vec![id_b.value];
        let killed = dyn_b.kill_multiple(&mut ids);

        assert!(ids.is_empty());

        links.kill_children(&killed);

        assert!(links.get_children(id_a).is_none());
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn validate_unupdated_parent_should_panic() {
        let links = DynDynOptMap::default();
        let mut dyn_a = Allocator::<DynA>::default();
        let dyn_b = Allocator::<DynB>::default();

        let id = dyn_a.create().value;
        dyn_a.kill(id);

        links.validate(&dyn_a, &dyn_b);
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn validate_unupdated_child_should_panic() {
        let links = DynDynOptMap::default();
        let dyn_a = Allocator::<DynA>::default();
        let mut dyn_b = Allocator::<DynB>::default();

        let id = dyn_b.create().value;
        dyn_b.kill(id);

        links.validate(&dyn_a, &dyn_b);
    }

    fn validate_updated_succeeds() {
        let mut links = DynDynOptMap::default();
        let mut dyn_a = Allocator::<DynA>::default();
        let mut dyn_b = Allocator::<DynB>::default();

        let id_a = dyn_a.create();
        let id_b = dyn_b.create();

        links.kill_parent(id_a);
        links.kill_child(id_b);

        let id_a = id_a.value;
        let id_b = id_b.value;

        dyn_a.kill(id_a);
        dyn_b.kill(id_b);

        links.validate(&dyn_a, &dyn_b);
    }

    #[test]
    fn size_test() {
        struct D1;
        impl Entity for D1 {
            type IdType = Dynamic;
        }
        struct D2;
        impl Entity for D2 {
            type IdType = Dynamic;
        }

        #[cfg(debug_assertions)]
        assert_eq!(
            64,
            std::mem::size_of::<RawLinks<D1, D2, RawComponent<D1, Vec<Id<D2>>>>>()
        );

        #[cfg(not(debug_assertions))]
        assert_eq!(
            48,
            std::mem::size_of::<RawLinks<D1, D2, RawComponent<D1, Vec<Id<D2>>>>>()
        );

        #[cfg(debug_assertions)]
        assert_eq!(
            72,
            std::mem::size_of::<RawLinks<D1, D2, RawIdMap<D1, Vec<Id<D2>>>>>()
        );

        #[cfg(not(debug_assertions))]
        assert_eq!(
            56,
            std::mem::size_of::<RawLinks<D1, D2, RawIdMap<D1, Vec<Id<D2>>>>>()
        );
    }

    #[test]
    fn range_links() {
        let mut links = FixFixRange::default();

        let p = Id::new(0, ());
        let c0 = Id::new(0, ());
        let c1 = Id::new(1, ());

        links.link(p, c0);
        links.link(p, c1);

        assert_eq!(vec![c0, c1], links[p].into_iter().collect::<Vec<_>>());
        assert_eq!(p, links[c0]);
        assert_eq!(p, links[c1]);

        // there should be no entries for these ids
        assert_eq!(None, links.get_children(Id::new(1, ())));
        assert_eq!(None, links.get_parent(Id::new(2, ())));
    }

    #[test]
    fn range_links_map() {
        let mut links = FixFixRangeMap::default();

        let p = Id::new(0, ());
        let c0 = Id::new(0, ());
        let c1 = Id::new(1, ());

        links.link(p, c0);
        links.link(p, c1);

        assert_eq!(vec![c0, c1], links[p].into_iter().collect::<Vec<_>>());
        assert_eq!(p, links[c0]);
        assert_eq!(p, links[c1]);

        // there should be no entries for these ids
        assert_eq!(None, links.get_children(Id::new(1, ())));
        assert_eq!(None, links.get_parent(Id::new(2, ())));
    }
}
