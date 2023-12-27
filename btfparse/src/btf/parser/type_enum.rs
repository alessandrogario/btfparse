use crate::btf::parser::{Int, Typedef};

/// An enum representing a BTF type
pub enum Type {
    Int(Int),
    Typedef(Typedef),
}
