use criterion::{Criterion, criterion_group, criterion_main};
use rand::RngExt;
use shrubbery::{shrubbery::ShrubberySettings, voxel::voxelize};
use std::hint::black_box;

#[inline]
fn build_and_voxelize(tree_asset: ShrubberySettings, seed: u64) {
    let mut generator = tree_asset.make_generator(seed);
    generator.execute_all_step(&tree_asset);
    let voxels = voxelize(&mut generator, &tree_asset);
}

pub fn load_all_shrubberies() -> Vec<ShrubberySettings> {
    let read_dir = std::fs::read_dir("assets/shrubbery").expect("folder");
    let mut shrubberies = vec![];
    for entry_result in read_dir {
        let dir_entry = entry_result.expect("entry");
        println!("found file: {:?}", dir_entry.file_name());

        let bytes = std::fs::read(dir_entry.path()).expect("valid file");
        let deserialized: ShrubberySettings = ron::de::from_bytes(&bytes).expect("valid ron");
        shrubberies.push(deserialized);
    }
    shrubberies
}

fn criterion_benchmark(c: &mut Criterion) {
    use rand::Rng;

    // LOAD ALL assets
    let shrubberies = load_all_shrubberies();

    // c.bench_function("1 sample", |b| {
    //     b.iter_with_setup(
    //         || {
    //             let mut rng = rand::rng();
    //             (
    //                 black_box(rng.random_range(-100.0..100.0)),
    //                 black_box(rng.random_range(-100.0..100.0)),
    //             )
    //         },
    //         |(x, y)| build_and_voxelize(x, y),
    //     )
    // });
    // c.bench_function("32x32 sample: surpass tinyvec", |b| {
    //     b.iter(|| sample_32x32(black_box(&worley_k_8)));
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
