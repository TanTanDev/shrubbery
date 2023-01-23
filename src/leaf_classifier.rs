#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LeafClassifier {
    // branch without children is leaf
    LastBranch,
    // new "generation" branches are leaves
    NonRootBranch,
}
