use crate::btf::parser::{Const, Enum32, Int, Ptr, Typedef};

/// An enum representing a BTF type
pub enum Type {
    /// An integer type
    Int(Int),

    /// A ptr type
    Ptr(Ptr),

    /// A typedef type
    Typedef(Typedef),

    /// A const type
    Const(Const),

    /// An 32-bit enum type
    Enum(Enum32),
}
