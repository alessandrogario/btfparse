use crate::btf::parser::{Enum32, Int, Typedef};

/// An enum representing a BTF type
pub enum Type {
    /// An integer type
    Int(Int),

    /// A typedef type
    Typedef(Typedef),

    /// An 32-bit enum type
    Enum(Enum32),
}
