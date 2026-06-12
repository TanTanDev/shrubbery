#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafClassifier {
    // branch without children is leaf
    LastBranch,
    // new "generation" branches are leaves
    NonRootBranch,
}
