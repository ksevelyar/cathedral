use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;
use terminal::maps::SandField;

fn benchmark_sand_field_generation(criterion: &mut Criterion) {
    let sand_field_measurement_duration = Duration::from_secs(10);
    let sand_field_sample_count = 20;
    let mut benchmark_group = criterion.benchmark_group("sandfield");
    benchmark_group.measurement_time(sand_field_measurement_duration);
    benchmark_group.sample_size(sand_field_sample_count);
    benchmark_group.bench_function("geometry_generation", |bencher| {
        bencher.iter(|| black_box(SandField::create(300.0).generate_geometry()));
    });
    benchmark_group.finish();
}

criterion_group!(sandfield, benchmark_sand_field_generation);
criterion_main!(sandfield);
