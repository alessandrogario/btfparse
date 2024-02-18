/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use std::ops::Add;

use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};

/// The location of a member
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offset {
    /// Byte offset
    ByteOffset(u32),

    /// Bit offset and size
    BitOffsetAndSize(u32, u32),
}

/// Implements the `Add<u32>` trait for `Offset`
impl Add<u32> for Offset {
    /// A `BTFResult<Offset>` type, because this operation can fail
    type Output = BTFResult<Self>;

    /// Adds the given value to the member offset. This operation will fail if `self` is a bitfields
    fn add(self, rhs: u32) -> Self::Output {
        match self {
            Offset::ByteOffset(byte_offset) => Ok(Offset::ByteOffset(byte_offset + rhs)),

            Offset::BitOffsetAndSize(_, _) => Err(BTFError::new(
                BTFErrorKind::UnexpectedBitfield,
                "Attempted to add a byte offset to a bitfield",
            )),
        }
    }
}

/// Implements the `Add<Offset>` trait for `Offset`
impl Add<Offset> for Offset {
    /// A `BTFResult<Offset>` type, because this operation can fail
    type Output = BTFResult<Self>;

    /// Adds the given `Offset` to `self`. This operation will fail if both `self` and `rhs` are bitfields
    fn add(self, rhs: Offset) -> Self::Output {
        match (self, rhs) {
            (Offset::ByteOffset(lhs_byte_offset), Offset::ByteOffset(rhs_byte_offset)) => {
                Ok(Offset::ByteOffset(lhs_byte_offset + rhs_byte_offset))
            }

            (
                Offset::ByteOffset(lhs_byte_offset),
                Offset::BitOffsetAndSize(rhs_bit_offset, rhs_bit_size),
            ) => {
                let member_offset =
                    Offset::BitOffsetAndSize((lhs_byte_offset * 8) + rhs_bit_offset, rhs_bit_size);

                Ok(member_offset)
            }

            _ => Err(BTFError::new(
                BTFErrorKind::UnexpectedBitfield,
                "Attempted to add a byte offset to a bitfield",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32_addition() {
        assert_eq!(Offset::ByteOffset(0).add(1).unwrap(), Offset::ByteOffset(1));
        assert!(Offset::BitOffsetAndSize(0, 0).add(1).is_err());
    }

    #[test]
    fn test_offset_addition() {
        assert_eq!(
            Offset::ByteOffset(0).add(Offset::ByteOffset(1)).unwrap(),
            Offset::ByteOffset(1)
        );

        assert_eq!(
            Offset::ByteOffset(0)
                .add(Offset::BitOffsetAndSize(1, 1))
                .unwrap(),
            Offset::BitOffsetAndSize(1, 1)
        );

        assert!(Offset::BitOffsetAndSize(0, 0)
            .add(Offset::ByteOffset(1))
            .is_err());

        assert!(Offset::BitOffsetAndSize(0, 0)
            .add(Offset::BitOffsetAndSize(1, 1))
            .is_err());
    }
}
