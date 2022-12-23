use std::path::Path;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mmt::multimedia::is_same_file;

pub fn criterion_benchmark(c: &mut Criterion) {
    let path_a = Path::new("./input/IMG_7705.CR2");
    let path_b = Path::new("./input/IMG_7705_copy.CR2");

    c.bench_function("same_file", |b| {
        b.iter(|| is_same_file(black_box(path_a), black_box(path_b)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
