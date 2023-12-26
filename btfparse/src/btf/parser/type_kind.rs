use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};

/// A BTF type kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    /// Integer
    Int = 1,

    /// Pointer
    Ptr = 2,

    /// Array
    Array = 3,

    /// Struct
    Struct = 4,

    /// Union
    Union = 5,

    /// Enumeration up to 32-bit values
    Enum = 6,

    /// Forward
    Fwd = 7,

    /// Typedef
    Typedef = 8,

    /// Volatile
    Volatile = 9,

    /// Const
    Const = 10,

    /// Restrict
    Restrict = 11,

    /// Function
    Func = 12,

    /// Function Prototype
    FuncProto = 13,

    /// Variable
    Var = 14,

    /// Section
    DataSec = 15,

    /// Floating point
    Float = 16,

    /// Decl Tag
    DeclTag = 17,

    /// Type Tag
    TypeTag = 18,

    /// Enumeration up to 64-bit values
    Enum64 = 19,
}

impl TypeKind {
    /// Converts a `u32` into a `TypeKind` value
    pub fn new(value: u32) -> BTFResult<TypeKind> {
        match value {
            1 => Ok(TypeKind::Int),
            2 => Ok(TypeKind::Ptr),
            3 => Ok(TypeKind::Array),
            4 => Ok(TypeKind::Struct),
            5 => Ok(TypeKind::Union),
            6 => Ok(TypeKind::Enum),
            7 => Ok(TypeKind::Fwd),
            8 => Ok(TypeKind::Typedef),
            9 => Ok(TypeKind::Volatile),
            10 => Ok(TypeKind::Const),
            11 => Ok(TypeKind::Restrict),
            12 => Ok(TypeKind::Func),
            13 => Ok(TypeKind::FuncProto),
            14 => Ok(TypeKind::Var),
            15 => Ok(TypeKind::DataSec),
            16 => Ok(TypeKind::Float),
            17 => Ok(TypeKind::DeclTag),
            18 => Ok(TypeKind::TypeTag),
            19 => Ok(TypeKind::Enum64),

            _ => Err(BTFError::new(
                BTFErrorKind::InvalidBTFKind,
                &format!("Invalid BTF kind value: 0x{:04X}", value),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypeKind;

    #[test]
    fn test_type_kind() {
        for i in 0..=20 {
            let type_kind_res = TypeKind::new(i);
            assert_eq!(type_kind_res.is_err(), i == 0 || i == 20);
        }
    }
}
