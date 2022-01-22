fn main() {}

#[derive(Debug)]
pub struct FixA;

impl gen_id::Entity for FixA {
    type IdType = gen_id::Static;
}

#[derive(Debug)]
pub struct DynA;

impl gen_id::Entity for DynA {
    type IdType = gen_id::Dynamic;
}

#[derive(Debug)]
pub struct FixB;

impl gen_id::Entity for FixB {
    type IdType = gen_id::Static;
}

#[derive(Debug)]
pub struct DynB;

impl gen_id::Entity for DynB {
    type IdType = gen_id::Dynamic;
}

// gen_id::dense_range_link!(FixFixRange, FixA, FixB);
// gen_id::sparse_range_link!(FixFixRangeMap, FixA, FixB);
//
// gen_id::dense_required_link!(FixFixReq, FixA: Fixed, FixB: Fixed);
// gen_id::dense_required_link!(FixDynReq, FixA: Fixed, DynB: Dynamic);
//
// gen_id::dense_optional_link!(DynDynOpt, DynA: Dynamic, DynB: Dynamic);
// gen_id::dense_optional_link!(DynFixOpt, DynA: Dynamic, FixB: Fixed);
// gen_id::dense_optional_link!(FixDynOpt, FixA: Fixed, DynB: Dynamic);
// gen_id::dense_optional_link!(FixFixOpt, FixA: Fixed, FixB: Fixed);
//
// gen_id::sparse_required_link!(FixFixReqMap, FixA: Fixed, FixB: Fixed);
// gen_id::sparse_required_link!(FixDynReqMap, FixA: Fixed, DynB: Dynamic);
//
// gen_id::sparse_optional_link!(DynDynOptMap, DynA: Dynamic, DynB: Dynamic);
// gen_id::sparse_optional_link!(DynFixOptMap, DynA: Dynamic, FixB: Fixed);
// gen_id::sparse_optional_link!(FixDynOptMap, FixA: Fixed, DynB: Dynamic);
// gen_id::sparse_optional_link!(FixFixOptMap, FixA: Fixed, FixB: Fixed);

#[test]
fn new_links() {
    use gen_id::id_map::RawIdMap;
    use gen_id::links::{Links, MapSet, Required};
    use gen_id::{Allocator, Id, RangeAllocator};

    let mut links = Links::<FixA, DynB, MapSet, Required>::default();
    let mut fix_a = RangeAllocator::<FixA>::default();
    let mut dyn_b = Allocator::<DynB>::default();

    let id_a = fix_a.create();
    links.link(id_a, dyn_b.create());

    let valid = links.validate_child(&dyn_b);
    // let set = &valid[id_a];
}
