use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::RngExt;
use shrubbery_voxel::prelude::{ShrubberyGenerator, ShrubberySettings};
use std::hint::black_box;

#[inline]
fn build_and_voxelize(tree_asset: &ShrubberySettings, seed: u64) {
    let mut generator = ShrubberyGenerator::generate(seed, tree_asset);
    let _voxels = generator.voxelize();
}

pub fn load_all_shrubberies() -> Vec<(String, ShrubberySettings)> {
    let read_dir = std::fs::read_dir("assets/shrubbery").expect("folder");
    let mut shrubberies = vec![];
    for entry_result in read_dir {
        let dir_entry = entry_result.expect("entry");
        let file_name = dir_entry.file_name();
        let file_name_string = dir_entry.file_name().to_string_lossy().into_owned();
        println!("found file: {:?}", file_name);

        let bytes = std::fs::read(dir_entry.path()).expect("valid file");
        let deserialized: ShrubberySettings = ron::de::from_bytes(&bytes).expect("valid ron");
        shrubberies.push((file_name_string, deserialized));
    }
    shrubberies
}

fn criterion_benchmark(c: &mut Criterion) {
    let shrubberies = load_all_shrubberies();

    c.bench_function("every tree", |b| {
        b.iter_with_setup(
            || {
                let mut rng = rand::rng();
                (black_box(rng.random::<u64>()),)
            },
            |(seed,)| {
                for (_name, asset) in shrubberies.iter() {
                    build_and_voxelize(asset, seed);
                }
            },
        )
    });

    // Create a benchmark group for organization
    let mut group = c.benchmark_group("seperate trees");

    for (name, settings) in &shrubberies {
        // We use BenchmarkId to cleanly differentiate them in the reports
        group.bench_with_input(BenchmarkId::new("voxelize", name), settings, |b, asset| {
            b.iter_with_setup(
                || {
                    let mut rng = rand::rng();
                    rng.random::<u64>()
                },
                |seed| build_and_voxelize(asset, black_box(seed)),
            );
        });
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
