/// Implement this trait for types to associate collections with that type.
pub trait Entity: std::fmt::Debug + 'static {
    type IdType: IdType;
}

#[cfg(feature = "serde")]
/// Defines the associated types for `Id<E>` and collections with an [`crate::gen::AllocGen<E>`] checksum value
pub trait IdType {
    type Gen: std::fmt::Debug
        + Copy
        + Eq
        + std::hash::Hash
        + Ord
        + Send
        + Sync
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>;
    type AllocGen: std::fmt::Debug
        + Default
        + Clone
        + Eq
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>
        + Send
        + Sync
        + 'static;
}
#[cfg(not(feature = "serde"))]
/// Defines the associated types for `Id<E>` and collections with an [`crate::gen::AllocGen<E>`] checksum value
pub trait IdType {
    type Gen: std::fmt::Debug + Copy + Eq + std::hash::Hash + Ord + Send + Sync;
    type AllocGen: std::fmt::Debug + Default + Clone + Eq + Send + Sync + 'static;
}

/// Entity types with an IdType of Static cannot be killed,
/// and static Ids do not need to be validated before indexing into a collection.
pub struct Static;

impl IdType for Static {
    type Gen = ();
    type AllocGen = ();
}

/// Entity types with an IdType of Dynamic can be created and killed,
/// and dynamic Ids need to be validated before they can be used to index into collections.
pub struct Dynamic;

impl IdType for Dynamic {
    type Gen = crate::gen::Gen;
    type AllocGen = u32;
}
