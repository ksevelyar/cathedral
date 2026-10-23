use cathedral::maps::{SandField, SandFieldOpening};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;

fn benchmark_sand_field_generation(criterion: &mut Criterion) {
    let sand_field_radius = 300.0;
    let tower_horizontal_position = bevy::prelude::Vec2::new(0.0, -72.0);
    let tower_terrain_opening_half_width = 17.5;
    let tower_terrain_opening = SandFieldOpening::create(
        tower_horizontal_position,
        bevy::prelude::Vec2::splat(tower_terrain_opening_half_width),
    );
    let sand_field = SandField::create(sand_field_radius).with_opening(tower_terrain_opening);
    let sand_field_measurement_duration = Duration::from_secs(10);
    let sand_field_sample_count = 20;
    let mut benchmark_group = criterion.benchmark_group("sandfield");
    benchmark_group.measurement_time(sand_field_measurement_duration);
    benchmark_group.sample_size(sand_field_sample_count);
    benchmark_group.bench_function("geometry_generation", |bencher| {
        bencher.iter(|| black_box(sand_field.generate_geometry()));
    });
    benchmark_group.finish();
}

criterion_group!(sandfield, benchmark_sand_field_generation);
criterion_main!(sandfield);
