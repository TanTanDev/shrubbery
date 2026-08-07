use glam::Vec3;
use shrubbery::{
    branch::Filter,
    prelude::*,
    shrubbery::{
        BranchThickness, GrowStep, InitialDir, SpawnRootStep, ValueOrRangeF32, ValueOrRangeU32,
    },
    voxel::{DecorationSelector, LeafDecoration},
};

fn main() {
    let shrubbery_settings = shrubbery_settings();
    let seed = rand::random();

    // generate a shrubbery "structure" from seed
    let mut generator = ShrubberyGenerator::generate(seed, &shrubbery_settings);
    println!("bounds: {:?}", generator.bounds());

    let voxels = generator.voxelize();
    println!("voxel count: {:?}", voxels.len());
}

// These build steps, produce branches growing 1 to 10 times upwards
fn shrubbery_settings() -> ShrubberySettings {
    let steps = [
        ShrubberyStep::SpawnRoot(SpawnRootStep {
            initial_dir: InitialDir::Value(Vec3::Y),
            ..Default::default()
        }),
        ShrubberyStep::Grow(GrowStep {
            times: ValueOrRangeU32::Range(1, 10),
            //  1 unit wide coverage
            thickness: BranchThickness::ValueOrRange(ValueOrRangeF32::Value(0.5)),
            voxel: DecorationSelector::Value(LeafDecoration::Solid(VoxelMapping {
                name: "dirt".to_string(),
                id: VoxelId(0),
            })),
            filter: Filter {
                ignore_root: false,
                ..Default::default()
            },
            ..Default::default()
        }),
    ];
    let mut shrubbery_settings = ShrubberySettings::default();
    shrubbery_settings.build_steps = steps.into_iter().collect();
    shrubbery_settings
}
