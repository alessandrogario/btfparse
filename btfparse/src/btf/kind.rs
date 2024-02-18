/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};

/// A BTF type kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
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

impl Kind {
    /// Converts a `u32` into a `TypeKind` value
    pub fn new(value: u32) -> BTFResult<Kind> {
        match value {
            1 => Ok(Kind::Int),
            2 => Ok(Kind::Ptr),
            3 => Ok(Kind::Array),
            4 => Ok(Kind::Struct),
            5 => Ok(Kind::Union),
            6 => Ok(Kind::Enum),
            7 => Ok(Kind::Fwd),
            8 => Ok(Kind::Typedef),
            9 => Ok(Kind::Volatile),
            10 => Ok(Kind::Const),
            11 => Ok(Kind::Restrict),
            12 => Ok(Kind::Func),
            13 => Ok(Kind::FuncProto),
            14 => Ok(Kind::Var),
            15 => Ok(Kind::DataSec),
            16 => Ok(Kind::Float),
            17 => Ok(Kind::DeclTag),
            18 => Ok(Kind::TypeTag),
            19 => Ok(Kind::Enum64),

            _ => Err(BTFError::new(
                BTFErrorKind::InvalidBTFKind,
                &format!("Invalid BTF kind value: 0x{:04X}", value),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kind;

    #[test]
    fn test_type_kind() {
        for i in 0..=20 {
            let type_kind_res = Kind::new(i);
            assert_eq!(type_kind_res.is_err(), i == 0 || i == 20);
        }
    }
}
