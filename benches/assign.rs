use criterion::{criterion_group, criterion_main, Criterion};

use gen_id::component::{Assign, TryAssign};
use gen_id::{Allocator, Component, Dynamic, Entity, ValidId};
use std::ops::AddAssign;

criterion_main! {
    assign_group
}

criterion_group! {
    assign_group,
    try_assign,
}

type V = f32;
const N: usize = 1024;

#[derive(Debug)]
struct Dyn;

impl Entity for Dyn {
    type IdType = Dynamic;
}

fn try_assign(c: &mut Criterion) {
    let mut comp = Component::<Dyn, V>::from(vec![0 as V; N]);
    let rhs = Component::<Dyn, V>::from((0..N).into_iter().map(|v| v as V).collect::<Vec<_>>());

    // only even indices have values
    let opt_rhs = Component::<Dyn, Option<V>>::from(
        (0..N)
            .into_iter()
            .map(|v| (v % 2 == 0).then_some(v as V))
            .collect::<Vec<_>>(),
    );

    // odd indices are dead
    let alloc = {
        let mut alloc = Allocator::<Dyn>::default();
        for _ in 0..N {
            let _ = alloc.create();
        }
        let ids = alloc.ids().map(|v| v.id()).collect::<Vec<_>>();
        for id in ids {
            if id.index() % 2 != 0 {
                alloc.kill(id);
            }
        }
        alloc
    };

    c.bench_function("assign", |b| b.iter(|| comp.assign(&rhs)))
        .bench_function("add_assign", |b| b.iter(|| comp.add_assign(&rhs)))
        .bench_function("try_assign", |b| b.iter(|| comp.try_assign(&opt_rhs)))
        .bench_function("try_assign_index_by_id", |b| {
            b.iter(|| {
                for id in alloc.ids() {
                    comp[id] = rhs[id];
                }
            })
        });
}
