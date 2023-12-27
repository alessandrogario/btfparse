use crate::btf::parser::{Array, Const, Enum32, Int, Ptr, Typedef, Volatile};

/// An enum representing a BTF type
pub enum Type {
    /// An integer type
    Int(Int),

    /// A ptr type
    Ptr(Ptr),

    /// An array type
    Array(Array),

    /// A typedef type
    Typedef(Typedef),

    /// A volatile type
    Volatile(Volatile),

    /// A const type
    Const(Const),

    /// An 32-bit enum type
    Enum(Enum32),
}
