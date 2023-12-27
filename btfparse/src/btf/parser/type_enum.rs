use crate::btf::parser::{Enum32, Int, Ptr, Typedef};

/// An enum representing a BTF type
pub enum Type {
    /// An integer type
    Int(Int),

    /// A ptr type
    Ptr(Ptr),

    /// A typedef type
    Typedef(Typedef),

    /// An 32-bit enum type
    Enum(Enum32),
}
