use std::hash::Hash;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use minecrust::atom::Atom2;
use rand::distr::{DistString, Standard};

fn bench_hstr(c: &mut Criterion) {
    let foobar = hstr::Atom::new("xxx_mod::something_block");
    let mut rng = rand::rng();
    c.bench_function("hstr_atom::new_same", |b| {
        b.iter(|| {
            let _ = black_box(hstr::Atom::new("xxx_mod::something_block"));
        })
    });
    c.bench_function("hstr_atom::new_random", |b| {
        b.iter_batched(
            || Standard.sample_string(&mut rng, 48),
            |data| {
                let _ = black_box(hstr::Atom::new(data));
            },
            BatchSize::SmallInput,
        )
    });
    c.bench_function("hstr_atom::clone", |b| {
        b.iter(|| {
            let _ = black_box(foobar.clone());
        });
    });
    c.bench_function("hstr_atom::eq", |b| {
        b.iter(|| {
            let _ = black_box(foobar == foobar);
        })
    });
}

fn bench_atom(c: &mut Criterion) {
    let foobar = Atom2::new("xxx_mod::something_block");
    let mut rng = rand::rng();
    c.bench_function("atom2::new_same", |b| {
        b.iter(|| {
            let _ = black_box(Atom2::new("xxx_mod::something_block"));
        })
    });
    c.bench_function("atom2::new_random", |b| {
        b.iter_batched(
            || Standard.sample_string(&mut rng, 48),
            |data| {
                let _ = black_box(Atom2::new(data));
            },
            BatchSize::SmallInput,
        )
    });
    c.bench_function("atom2::clone", |b| {
        b.iter(|| {
            let _ = black_box(foobar.clone());
        });
    });
    c.bench_function("atom2::eq", |b| {
        b.iter(|| {
            let _ = black_box(foobar == foobar);
        })
    });
}

criterion_group!(benches, bench_hstr, bench_atom);
criterion_main!(benches);
