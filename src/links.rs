#![allow(unused_macros)]

use crate::component::{Component, RawComponent};
use crate::hash::HashSet;
use crate::id_map::RawIdMap;
#[cfg(debug_assertions)]
use crate::AllocGen;
use crate::{
    Dynamic, Entity, GetMut, Id, IdRange, Insert, Killed, RefCast, Static, Valid, ValidId,
    Validator,
};
use iter_context::ContextualIterator;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct RawLinks<Parent: Entity, Child: Entity, Children: ChildrenTrait> {
    #[cfg(debug_assertions)]
    pub parent_gen: AllocGen<Parent>,
    #[cfg(debug_assertions)]
    pub child_gen: AllocGen<Child>,

    pub parent: RawComponent<Child, Option<Id<Parent>>>,
    pub children: Children::Type<Parent, Child>,
}

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait> Clone
    for RawLinks<Parent, Child, Children>
{
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

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait> Default
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

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait> RawLinks<Parent, Child, Children> {
    pub fn links<LinkType>(&self) -> &Links<Parent, Child, Children, LinkType> {
        Links::ref_cast(self)
    }
}

mod traits {
    use super::*;
    use crate::{GetMut, Insert, Remove};

    impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, Inner>
        RawLinks<Parent, Child, Children>
    where
        <Children as ChildrenTrait>::Type<Parent, Child>:
            GetMut<Id<Parent>, Value = Inner> + Insert<Id<Parent>, Value = Inner>,
        Inner: Insert<Id<Child>, Value = ()> + Default,
    {
        pub fn link_new(&mut self, parent: Id<Parent>, child: Id<Child>) {
            assert!(matches!(self.parent.get(child), None | Some(None)));
            match self.children.get_mut(parent) {
                Some(children) => {
                    children.insert(child, ());
                }
                None => {
                    let mut set = Inner::default();
                    set.insert(child, ());
                    self.children.insert(parent, set);
                }
            }
        }
    }

    impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, Inner>
        RawLinks<Parent, Child, Children>
    where
        <Children as ChildrenTrait>::Type<Parent, Child>:
            GetMut<Id<Parent>, Value = Inner> + Insert<Id<Parent>, Value = Inner>,
        Inner: Insert<Id<Child>, Value = ()> + Default + Remove<Id<Child>>,
    {
        pub fn relink(&mut self, parent: Id<Parent>, child: Id<Child>) {
            self.unlink1(child);
            match self.children.get_mut(parent) {
                Some(children) => {
                    children.insert(child, ());
                }
                None => {
                    let mut set = Inner::default();
                    set.insert(child, ());
                    self.children.insert(parent, set);
                }
            }
        }
    }

    impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, Inner>
        RawLinks<Parent, Child, Children>
    where
        <Children as ChildrenTrait>::Type<Parent, Child>: GetMut<Id<Parent>, Value = Inner>,
        Inner: Remove<Id<Child>>,
    {
        pub fn unlink1(&mut self, child: Id<Child>) {
            if let Some(parent) = self.parent.get_mut(child) {
                if let Some(parent) = parent.take() {
                    if let Some(children) = self.children.get_mut(parent) {
                        children.remove(&child);
                    }
                }
            }
        }
    }
}

impl<Parent: Entity, Child: Entity> RawLinks<Parent, Child, CompSet> {
    pub fn link(&mut self, parent: Id<Parent>, child: Id<Child>) {
        self.unlink(child);
        self.parent.insert_with(child, Some(parent), || None);
        match self.children.get_mut(parent) {
            Some(children) => {
                children.insert(child);
            }
            None => {
                let mut set = HashSet::default();
                set.insert(child);
                self.children.insert_with(parent, set, Default::default);
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

    pub fn get_children(&self, parent: Id<Parent>) -> Option<&HashSet<Id<Child>>> {
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

impl<Parent: Entity, Child: Entity> RawLinks<Parent, Child, MapSet> {
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

    pub fn get_children(&self, parent: Id<Parent>) -> Option<&HashSet<Id<Child>>> {
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

impl<Parent: Entity<IdType = Static>, Child: Entity<IdType = Static>>
    RawLinks<Parent, Child, CompRange>
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

impl<Parent: Entity<IdType = Static>, Child: Entity<IdType = Static>>
    RawLinks<Parent, Child, MapRange>
{
    pub fn link(&mut self, parent: Id<Parent>, child: Id<Child>) {
        self.parent.insert_with(child, Some(parent), || None);
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

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait> RawLinks<Parent, Child, Children> {
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

#[repr(transparent)]
#[derive(ref_cast::RefCast)]
pub struct Parents<Parent: Entity, Child: Entity> {
    parents: RawComponent<Child, Option<Id<Parent>>>,
}

impl<Parent: Entity, Child: Entity> Parents<Parent, Child> {
    pub fn new(parents: &RawComponent<Child, Option<Id<Parent>>>) -> &Self {
        ref_cast::RefCast::ref_cast(parents)
    }
}

impl<'a, Parent: Entity, Child: Entity> IntoIterator for &'a Parents<Parent, Child> {
    type Item = &'a Id<Parent>;
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.parents.into_iter().map(|opt| opt.as_ref().unwrap())
    }
}

impl<'a, Parent: Entity, Child: Entity> ContextualIterator for &'a Parents<Parent, Child> {
    type Context = Child;
}

#[derive(Debug, Default, Clone)]
pub struct Required;

#[derive(Debug, Default, Clone)]
pub struct Optional;

#[derive(Debug, Default, Clone)]
pub struct CompSet;

#[derive(Debug, Default, Clone)]
pub struct CompRange;

#[derive(Debug, Default, Clone)]
pub struct MapSet;

#[derive(Debug, Default, Clone)]
pub struct MapRange;

pub trait ChildrenTrait {
    type Type<Parent: Entity, Child: Entity>: Default + Clone;
}

impl ChildrenTrait for CompSet {
    type Type<Parent: Entity, Child: Entity> = RawComponent<Parent, HashSet<Id<Child>>>;
}

impl ChildrenTrait for CompRange {
    type Type<Parent: Entity, Child: Entity> = RawComponent<Parent, IdRange<Child>>;
}

impl ChildrenTrait for MapSet {
    type Type<Parent: Entity, Child: Entity> = RawIdMap<Parent, HashSet<Id<Child>>>;
}

impl ChildrenTrait for MapRange {
    type Type<Parent: Entity, Child: Entity> = RawIdMap<Parent, IdRange<Child>>;
}

#[repr(transparent)]
pub struct Links<Parent: Entity, Child: Entity, Children: ChildrenTrait, LinkType> {
    links: RawLinks<Parent, Child, Children>,
    marker: PhantomData<*const LinkType>,
}

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, LinkType> RefCast
    for Links<Parent, Child, Children, LinkType>
{
    type From = RawLinks<Parent, Child, Children>;

    fn ref_cast(from: &Self::From) -> &Self {
        let ptr = from as *const Self::From as *const Self;
        unsafe { &*ptr }
    }

    fn ref_cast_mut(from: &mut Self::From) -> &mut Self {
        let ptr = from as *mut Self::From as *mut Self;
        unsafe { &mut *ptr }
    }
}

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, ParentType> Default
    for Links<Parent, Child, Children, ParentType>
{
    fn default() -> Self {
        Self {
            links: Default::default(),
            marker: PhantomData,
        }
    }
}

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, ParentType> Clone
    for Links<Parent, Child, Children, ParentType>
{
    fn clone(&self) -> Self {
        Self {
            links: self.links.clone(),
            marker: PhantomData,
        }
    }
}

mod traits1 {
    use super::*;
    use crate::Remove;

    impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, LinkType, Inner>
        Links<Parent, Child, Children, LinkType>
    where
        <Children as ChildrenTrait>::Type<Parent, Child>:
            GetMut<Id<Parent>, Value = Inner> + Insert<Id<Parent>, Value = Inner>,
        Inner: Default + Insert<Id<Child>, Value = ()>,
    {
        pub fn link_new<P: ValidId<Entity = Parent>, C: ValidId<Entity = Child>>(
            &mut self,
            parent: P,
            child: C,
        ) {
            self.links.link_new(parent.id(), child.id());
        }
    }

    impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, LinkType, Inner>
        Links<Parent, Child, Children, LinkType>
    where
        <Children as ChildrenTrait>::Type<Parent, Child>:
            GetMut<Id<Parent>, Value = Inner> + Insert<Id<Parent>, Value = Inner>,
        Inner: Remove<Id<Child>>,
    {
        pub fn unlink1<C: ValidId<Entity = Child>>(&mut self, child: C) {
            self.links.unlink1(child.id());
        }
    }

    impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, LinkType, Inner>
        Links<Parent, Child, Children, LinkType>
    where
        <Children as ChildrenTrait>::Type<Parent, Child>:
            GetMut<Id<Parent>, Value = Inner> + Insert<Id<Parent>, Value = Inner>,
        Inner: Remove<Id<Child>> + Default + Insert<Id<Child>, Value = ()>,
    {
        pub fn relink<P: ValidId<Entity = Parent>, C: ValidId<Entity = Child>>(
            &mut self,
            parent: P,
            child: C,
        ) {
            self.links.relink(parent.id(), child.id());
        }
    }
}

impl<Parent: Entity, Child: Entity, LinkType> Links<Parent, Child, CompSet, LinkType> {
    pub fn link<P: ValidId<Entity = Parent>, C: ValidId<Entity = Child>>(
        &mut self,
        parent: P,
        child: C,
    ) {
        self.links.link(parent.id(), child.id());
    }
}

impl<Parent: Entity, Child: Entity, LinkType> Links<Parent, Child, MapSet, LinkType> {
    pub fn link<P: ValidId<Entity = Parent>, C: ValidId<Entity = Child>>(
        &mut self,
        parent: P,
        child: C,
    ) {
        self.links.link(parent.id(), child.id());
    }
}

impl<Parent: Entity, Child: Entity> Links<Parent, Child, CompSet, Optional> {
    pub fn unlink<C: ValidId<Entity = Child>>(&mut self, child: C) {
        self.links.unlink(child.id());
    }
}

impl<Parent: Entity<IdType = Dynamic>, Child: Entity, LinkType>
    Links<Parent, Child, CompSet, LinkType>
{
    pub fn kill_parent<P: ValidId<Entity = Parent>>(&mut self, parent: P) {
        self.links.kill_parent(parent.id());
    }
}

impl<Parent: Entity, Child: Entity<IdType = Dynamic>, LinkType>
    Links<Parent, Child, CompSet, LinkType>
{
    pub fn kill_child<C: ValidId<Entity = Child>>(&mut self, child: C) {
        self.links.kill_child(child.id());
    }
}

impl<Parent: Entity, Child: Entity> Links<Parent, Child, MapSet, Optional> {
    pub fn unlink<C: ValidId<Entity = Child>>(&mut self, child: C) {
        self.links.unlink(child.id());
    }
}

impl<Parent: Entity<IdType = Dynamic>, Child: Entity, LinkType>
    Links<Parent, Child, MapSet, LinkType>
{
    pub fn kill_parent<P: ValidId<Entity = Parent>>(&mut self, parent: P) {
        self.links.kill_parent(parent.id());
    }
}

impl<Parent: Entity, Child: Entity<IdType = Dynamic>, LinkType>
    Links<Parent, Child, MapSet, LinkType>
{
    pub fn kill_child<C: ValidId<Entity = Child>>(&mut self, child: C) {
        self.links.kill_child(child.id());
    }
}

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait>
    Links<Parent, Child, Children, Required>
{
    pub fn parents(&self) -> &Parents<Parent, Child> {
        Parents::new(&self.links.parent)
    }
}

impl<'v, Parent: Entity, Child: Entity, Children: ChildrenTrait>
    Valid<'v, Links<Parent, Child, Children, Required>>
{
    pub fn parents(&self) -> &Valid<'v, Parents<Parent, Child>> {
        Valid::new_ref(self.value.parents())
    }
}

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait>
    Links<Parent, Child, Children, Optional>
{
    pub fn parents(&self) -> &Component<Child, Option<Id<Parent>>> {
        self.links.parent.component()
    }
}

impl<'v, Parent: Entity, Child: Entity, Children: ChildrenTrait>
    Valid<'v, Links<Parent, Child, Children, Optional>>
{
    pub fn parents(&self) -> &Valid<'v, Component<Child, Option<Id<Parent>>>> {
        Valid::new_ref(self.value.parents())
    }
}

impl<Parent: Entity, Child: Entity, Children: ChildrenTrait, LinkType>
    Links<Parent, Child, Children, LinkType>
{
    pub fn children(&self) -> &Children::Type<Parent, Child> {
        &self.links.children
    }
}

impl<'v, Parent: Entity, Child: Entity, Children: ChildrenTrait, LinkType>
    Valid<'v, Links<Parent, Child, Children, LinkType>>
{
    pub fn children(&self) -> &Valid<Children::Type<Parent, Child>> {
        Valid::new_ref(self.value.children())
    }
}

impl<
        'v,
        Parent: Entity<IdType = Dynamic>,
        Child: Entity<IdType = Dynamic>,
        Children: ChildrenTrait,
        LinkType,
    > Links<Parent, Child, Children, LinkType>
{
    pub fn validate<P: Validator<'v, Parent>, C: Validator<'v, Child>>(
        &self,
        p: P,
        c: C,
    ) -> &Valid<'v, Self> {
        Valid::new_ref(self.links.validate_both(p, c).value.links())
    }
}

impl<
        'v,
        Parent: Entity<IdType = Dynamic>,
        Child: Entity<IdType = Static>,
        Children: ChildrenTrait,
        LinkType,
    > Links<Parent, Child, Children, LinkType>
{
    pub fn validate_parent<P: Validator<'v, Parent>>(&self, p: P) -> &Valid<'v, Self> {
        Valid::new_ref(self.links.validate_parent(p).value.links())
    }
}

impl<
        'v,
        Parent: Entity<IdType = Static>,
        Child: Entity<IdType = Dynamic>,
        Children: ChildrenTrait,
        LinkType,
    > Links<Parent, Child, Children, LinkType>
{
    pub fn validate_child<C: Validator<'v, Child>>(&self, c: C) -> &Valid<'v, Self> {
        Valid::new_ref(self.links.validate_child(c).value.links())
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
            ) -> std::option::Option<&$crate::IdRange<$child>> {
                self.links.get_children(parent)
            }

            pub fn get_parent(
                &self,
                child: $crate::Id<$child>,
            ) -> std::option::Option<&$crate::Id<$parent>> {
                self.links.get_parent(child)
            }

            pub fn parents(&self) -> &$crate::links::Parents<$parent, $child> {
                $crate::links::Parents::new(&self.links.parent)
            }

            pub fn children(
                &self,
            ) -> &$crate::component::Component<$parent, $crate::IdRange<$child>> {
                self.links.children.component()
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
            ) -> std::option::Option<&$crate::IdRange<$child>> {
                self.links.get_children(parent)
            }

            pub fn get_parent(
                &self,
                child: $crate::Id<$child>,
            ) -> std::option::Option<&$crate::Id<$parent>> {
                self.links.get_parent(child)
            }

            pub fn parents(&self) -> &$crate::links::Parents<$parent, $child> {
                $crate::links::Parents::new(&self.links.parent)
            }

            pub fn children(&self) -> &$crate::id_map::IdMap<$parent, $crate::IdRange<$child>> {
                self.links.children.id_map()
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
        // $crate::links_trait!($ty, $parent, $child, Required);
    };
    ($ty:ident, $parent:ident: Fixed, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: Component, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Required);
        // $crate::links_trait!($ty, $parent, $child, Required);

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
        // $crate::links_trait!($ty, $parent, $child, Optional);

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
        // $crate::links_trait!($ty, $parent, $child, Optional);

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
        // $crate::links_trait!($ty, $parent, $child, Optional);

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
        // $crate::links_trait!($ty, $parent, $child, Required);
    };
    ($ty:ident, $parent:ident: Fixed, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Required);
        // $crate::links_trait!($ty, $parent, $child, Required);

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
        // $crate::links_trait!($ty, $parent, $child, Optional);

        impl $ty {
            $crate::unlink!($parent, $child);
        }
    };
    ($ty:ident, $parent:ident: Fixed, $child:ident: Dynamic) => {
        $crate::define_links_inner!($ty, $parent: IdMap, $child);

        $crate::index_parent!($ty, $parent: Fixed, $child);
        $crate::index_child!($ty, $parent, $child: Dynamic, Optional);
        // $crate::links_trait!($ty, $parent, $child, Optional);

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
        // $crate::links_trait!($ty, $parent, $child, Optional);

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
        // $crate::links_trait!($ty, $parent, $child, Optional);

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
                $crate::component::RawComponent<$parent, $crate::hash::HashSet<$crate::Id<$child>>>,
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
                self.links
                    .link($crate::ValidId::id(parent), $crate::ValidId::id(child));
            }

            pub fn get_children<P: $crate::ValidId<Entity = $parent>>(
                &self,
                parent: P,
            ) -> std::option::Option<&$crate::hash::HashSet<$crate::Id<$child>>> {
                self.links.get_children($crate::ValidId::id(parent))
            }

            pub fn get_parent<C: $crate::ValidId<Entity = $child>>(
                &self,
                child: C,
            ) -> std::option::Option<&$crate::Id<$parent>> {
                self.links.get_parent($crate::ValidId::id(child))
            }

            pub fn children(
                &self,
            ) -> &$crate::component::Component<$parent, $crate::hash::HashSet<$crate::Id<$child>>>
            {
                self.links.children.component()
            }
        }
    };
    ($ty:ident, $parent:ident: IdMap, $child:ident) => {
        #[derive(Debug, Default, Clone)]
        pub struct $ty {
            links: $crate::links::RawLinks<
                $parent,
                $child,
                $crate::id_map::RawIdMap<$parent, $crate::hash::HashSet<$crate::Id<$child>>>,
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
                self.links
                    .link($crate::ValidId::id(parent), $crate::ValidId::id(child));
            }

            pub fn get_children<P: $crate::ValidId<Entity = $parent>>(
                &self,
                parent: P,
            ) -> std::option::Option<&$crate::hash::HashSet<$crate::Id<$child>>> {
                self.links.get_children($crate::ValidId::id(parent))
            }

            pub fn get_parent<C: $crate::ValidId<Entity = $child>>(
                &self,
                child: C,
            ) -> std::option::Option<&$crate::Id<$parent>> {
                self.links.get_parent($crate::ValidId::id(child))
            }

            pub fn children(
                &self,
            ) -> &$crate::id_map::IdMap<$parent, $crate::hash::HashSet<$crate::Id<$child>>> {
                self.links.children.id_map()
            }
        }
    };
}

#[macro_export]
macro_rules! index_parent {
    ($ty:ident, $parent:ident: Fixed, $child:ident) => {
        impl std::ops::Index<$crate::Id<$parent>> for $ty {
            type Output = $crate::hash::HashSet<$crate::Id<$child>>;
            fn index(&self, index: $crate::Id<$parent>) -> &Self::Output {
                self.links.children.index(index)
            }
        }

        impl std::ops::Index<&$crate::Id<$parent>> for $ty {
            type Output = $crate::hash::HashSet<$crate::Id<$child>>;
            fn index(&self, index: &$crate::Id<$parent>) -> &Self::Output {
                self.links.children.index(*index)
            }
        }
    };
    ($ty:ident, $parent:ident: Dynamic, $child:ident) => {
        impl std::ops::Index<$crate::Valid<'_, $crate::Id<$parent>>> for $ty {
            type Output = $crate::hash::HashSet<$crate::Id<$child>>;
            fn index(&self, index: $crate::Valid<$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index($crate::ValidId::id(index))
            }
        }

        impl std::ops::Index<$crate::Valid<'_, &$crate::Id<$parent>>> for $ty {
            type Output = $crate::hash::HashSet<$crate::Id<$child>>;
            fn index(&self, index: $crate::Valid<&$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index($crate::ValidId::id(index))
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, $crate::Id<$parent>>> for $ty {
            type Output = $crate::hash::HashSet<$crate::Id<$child>>;
            fn index(&self, index: &$crate::Valid<$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index($crate::ValidId::id(index))
            }
        }

        impl std::ops::Index<&$crate::Valid<'_, &$crate::Id<$parent>>> for $ty {
            type Output = $crate::hash::HashSet<$crate::Id<$child>>;
            fn index(&self, index: &$crate::Valid<&$crate::Id<$parent>>) -> &Self::Output {
                self.links.children.index($crate::ValidId::id(index))
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
            self.links.unlink($crate::ValidId::id(child));
        }
    };
}

#[macro_export]
macro_rules! kill_parent {
    ($parent:ident) => {
        pub fn kill_parent<P: $crate::ValidId<Entity = $parent>>(&mut self, parent: P) {
            self.links.kill_parent($crate::ValidId::id(parent));
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
            self.links.kill_child($crate::ValidId::id(child));
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
            $crate::Valid::new_ref(self)
        }
    };
    ($parent:ident: Dynamic, $child:ident: Fixed) => {
        #[allow(unused_variables)]
        pub fn validate<'v, V: $crate::Validator<'v, $parent>>(
            &self,
            v: V,
        ) -> &$crate::Valid<'v, Self> {
            let _ = self.links.validate_parent(v);
            $crate::Valid::new_ref(self)
        }
    };
    ($parent:ident: Dynamic, $child:ident: Dynamic) => {
        #[allow(unused_variables)]
        pub fn validate<'v, P: $crate::Validator<'v, $parent>, C: $crate::Validator<'v, $child>>(
            &self,
            p: P,
            c: C,
        ) -> &$crate::Valid<'v, Self> {
            #[cfg(debug_assertions)]
            {
                assert!(&self.links.parent_gen == p.as_ref());
                assert!(&self.links.child_gen == c.as_ref());
            }

            $crate::Valid::new_ref(self)
        }
    };
}

#[macro_export]
macro_rules! links_trait {
    ($ty:ident, $parent:ident: Component, $child:ident, Required) => {
        impl $crate::links::LinksTrait for $ty {
            // type Parents = $crate::links::Parents<$parent, $child>;
            // type Children =
            //     $crate::id_map::IdMap<$parent, $crate::hash::HashSet<$crate::Id<$child>>>;
            // fn parents(&self) -> &Self::Parents {
            //     todo!()
            // }
            // fn children(&self) -> &Self::Children {
            //     todo!()
            // }
        }
    };

    ($ty:ident, $parent:ident: Component, $child:ident, Optional) => {
        // impl AsRef<$crate::component::Component<$child, std::option::Option<$crate::Id<$parent>>>>
        //     for $ty
        // {
        //     fn as_ref(
        //         &self,
        //     ) -> &$crate::component::Component<$child, std::option::Option<$crate::Id<$parent>>>
        //     {
        //         &self.links.parent.component()
        //     }
        // }
        //
        // impl $ty {
        //     pub fn parents(
        //         &self,
        //     ) -> &$crate::component::Component<$child, std::option::Option<$crate::Id<$parent>>>
        //     {
        //         self.links.parent.component()
        //     }
        // }
    };

    ($ty:ident, $parent:ident: IdMap, $child:ident, Required) => {
        // impl $crate::links::LinksTrait for $ty {
        //     type Parents = $crate::links::Parents<$parent, $child>;
        //     type Children = $children;
        //     fn parents(&self) -> &Self::Parents {
        //         todo!()
        //     }
        //     fn children(&self) -> &Self::Children {
        //         todo!()
        //     }
        // }
    };

    ($ty:ident, $parent:ident: IdMap, $child:ident, Optional) => {
        // impl AsRef<$crate::component::Component<$child, std::option::Option<$crate::Id<$parent>>>>
        //     for $ty
        // {
        //     fn as_ref(
        //         &self,
        //     ) -> &$crate::component::Component<$child, std::option::Option<$crate::Id<$parent>>>
        //     {
        //         &self.links.parent.component()
        //     }
        // }
        //
        // impl $ty {
        //     pub fn parents(
        //         &self,
        //     ) -> &$crate::component::Component<$child, std::option::Option<$crate::Id<$parent>>>
        //     {
        //         self.links.parent.component()
        //     }
        // }
    };
}

#[cfg(test)]
mod test {
    #![allow(dead_code)]

    use crate::{Dynamic, Entity, Static};

    #[derive(Debug)]
    pub struct FixA;

    impl Entity for FixA {
        type IdType = Static;
    }

    #[derive(Debug)]
    pub struct DynA;

    impl Entity for DynA {
        type IdType = Dynamic;
    }

    #[derive(Debug)]
    pub struct FixB;

    impl Entity for FixB {
        type IdType = Static;
    }

    #[derive(Debug)]
    pub struct DynB;

    impl Entity for DynB {
        type IdType = Dynamic;
    }

    // dense_range_link!(FixFixRange, FixA, FixB);
    // sparse_range_link!(FixFixRangeMap, FixA, FixB);
    //
    // dense_required_link!(FixFixReq, FixA: Fixed, FixB: Fixed);
    // dense_required_link!(FixDynReq, FixA: Fixed, DynB: Dynamic);
    //
    // dense_optional_link!(DynDynOpt, DynA: Dynamic, DynB: Dynamic);
    // dense_optional_link!(DynFixOpt, DynA: Dynamic, FixB: Fixed);
    // dense_optional_link!(FixDynOpt, FixA: Fixed, DynB: Dynamic);
    // dense_optional_link!(FixFixOpt, FixA: Fixed, FixB: Fixed);
    //
    // sparse_required_link!(FixFixReqMap, FixA: Fixed, FixB: Fixed);
    // sparse_required_link!(FixDynReqMap, FixA: Fixed, DynB: Dynamic);
    //
    // sparse_optional_link!(DynDynOptMap, DynA: Dynamic, DynB: Dynamic);
    // sparse_optional_link!(DynFixOptMap, DynA: Dynamic, FixB: Fixed);
    // sparse_optional_link!(FixDynOptMap, FixA: Fixed, DynB: Dynamic);
    // sparse_optional_link!(FixFixOptMap, FixA: Fixed, FixB: Fixed);
    //
    // #[test]
    // fn test() {
    //     let mut links = DynDynOptMap::default();
    //     let mut dyn_a = Allocator::<DynA>::default();
    //     let mut dyn_b = Allocator::<DynB>::default();
    //
    //     let id_a = dyn_a.create();
    //     let id_b = dyn_b.create();
    //
    //     links.link(id_a, id_b);
    //
    //     assert!(links[id_a].contains(&id_b.id()));
    //     assert_eq!(Some(id_a.id()), links[id_b]);
    //
    //     links.unlink(id_b);
    //
    //     assert!(links.get_children(id_a).is_none());
    //     assert_eq!(None, links[id_b]);
    //
    //     let mut ids = vec![id_b.value];
    //     let killed = dyn_b.kill_multiple(&mut ids);
    //
    //     assert!(ids.is_empty());
    //
    //     links.kill_children(&killed);
    //
    //     assert!(links.get_children(id_a).is_none());
    // }
    //
    // #[test]
    // #[should_panic]
    // #[cfg(debug_assertions)]
    // fn validate_unupdated_parent_should_panic() {
    //     let links = DynDynOptMap::default();
    //     let mut dyn_a = Allocator::<DynA>::default();
    //     let dyn_b = Allocator::<DynB>::default();
    //
    //     let id = dyn_a.create().value;
    //     dyn_a.kill(id);
    //
    //     links.validate(&dyn_a, &dyn_b);
    // }
    //
    // #[test]
    // #[should_panic]
    // #[cfg(debug_assertions)]
    // fn validate_unupdated_child_should_panic() {
    //     let links = DynDynOptMap::default();
    //     let dyn_a = Allocator::<DynA>::default();
    //     let mut dyn_b = Allocator::<DynB>::default();
    //
    //     let id = dyn_b.create().value;
    //     dyn_b.kill(id);
    //
    //     links.validate(&dyn_a, &dyn_b);
    // }
    //
    // fn validate_updated_succeeds() {
    //     let mut links = DynDynOptMap::default();
    //     let mut dyn_a = Allocator::<DynA>::default();
    //     let mut dyn_b = Allocator::<DynB>::default();
    //
    //     let id_a = dyn_a.create();
    //     let id_b = dyn_b.create();
    //
    //     links.kill_parent(id_a);
    //     links.kill_child(id_b);
    //
    //     let id_a = id_a.value;
    //     let id_b = id_b.value;
    //
    //     dyn_a.kill(id_a);
    //     dyn_b.kill(id_b);
    //
    //     links.validate(&dyn_a, &dyn_b);
    // }
    //
    // #[test]
    // fn size_test() {
    //     struct D1;
    //     impl Entity for D1 {
    //         type IdType = Dynamic;
    //     }
    //     struct D2;
    //     impl Entity for D2 {
    //         type IdType = Dynamic;
    //     }
    //
    //     #[cfg(debug_assertions)]
    //     assert_eq!(
    //         64,
    //         std::mem::size_of::<RawLinks<D1, D2, RawComponent<D1, Vec<Id<D2>>>>>()
    //     );
    //
    //     #[cfg(not(debug_assertions))]
    //     assert_eq!(
    //         48,
    //         std::mem::size_of::<RawLinks<D1, D2, RawComponent<D1, Vec<Id<D2>>>>>()
    //     );
    //
    //     #[cfg(debug_assertions)]
    //     assert_eq!(
    //         80,
    //         std::mem::size_of::<RawLinks<D1, D2, RawIdMap<D1, Vec<Id<D2>>>>>()
    //     );
    //
    //     #[cfg(not(debug_assertions))]
    //     assert_eq!(
    //         56,
    //         std::mem::size_of::<RawLinks<D1, D2, RawIdMap<D1, Vec<Id<D2>>>>>()
    //     );
    // }
    //
    // #[test]
    // fn range_links() {
    //     let mut links = FixFixRange::default();
    //
    //     let p = Id::new(0, ());
    //     let c0 = Id::new(0, ());
    //     let c1 = Id::new(1, ());
    //
    //     links.link(p, c0);
    //     links.link(p, c1);
    //
    //     assert_eq!(vec![c0, c1], links[p].into_iter().collect::<Vec<_>>());
    //     assert_eq!(p, links[c0]);
    //     assert_eq!(p, links[c1]);
    //
    //     // there should be no entries for these ids
    //     assert_eq!(None, links.get_children(Id::new(1, ())));
    //     assert_eq!(None, links.get_parent(Id::new(2, ())));
    // }
    //
    // #[test]
    // fn range_links_map() {
    //     let mut links = FixFixRangeMap::default();
    //
    //     let p = Id::new(0, ());
    //     let c0 = Id::new(0, ());
    //     let c1 = Id::new(1, ());
    //
    //     links.link(p, c0);
    //     links.link(p, c1);
    //
    //     assert_eq!(vec![c0, c1], links[p].into_iter().collect::<Vec<_>>());
    //     assert_eq!(p, links[c0]);
    //     assert_eq!(p, links[c1]);
    //
    //     // there should be no entries for these ids
    //     assert_eq!(None, links.get_children(Id::new(1, ())));
    //     assert_eq!(None, links.get_parent(Id::new(2, ())));
    // }
    //
    // #[test]
    // fn iter_test() {
    //     use iter_context::ContextualIterator;
    //
    //     let link = FixFixRange::default();
    //     let comp = crate::component::Component::<FixB, ()>::default();
    //
    //     comp.iter().zip(link.parents()).for_each(|(u, l)| {});
    //
    //     let d = DynDynOpt::default();
    //     let a = Allocator::default();
    //     let b = Allocator::default();
    //     let v = d.validate(&a, &b);
    //     // let p = v.as_ref();
    //     // p.zip(p).for_each(|(a, b)| {});
    // }
}
