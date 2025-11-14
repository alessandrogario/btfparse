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
            Offset::ByteOffset(byte_offset) => Ok(Offset::ByteOffset(
                byte_offset.checked_add(rhs).ok_or_else(|| {
                    BTFError::new(BTFErrorKind::InvalidOffset, "Byte offset addition overflow")
                })?,
            )),

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
            (Offset::ByteOffset(lhs_byte_offset), Offset::ByteOffset(rhs_byte_offset)) => Ok(
                Offset::ByteOffset(lhs_byte_offset.checked_add(rhs_byte_offset).ok_or_else(
                    || BTFError::new(BTFErrorKind::InvalidOffset, "Byte offset addition overflow"),
                )?),
            ),

            (
                Offset::ByteOffset(lhs_byte_offset),
                Offset::BitOffsetAndSize(rhs_bit_offset, rhs_bit_size),
            ) => {
                let bit_offset = lhs_byte_offset
                    .checked_mul(8)
                    .ok_or_else(|| {
                        BTFError::new(
                            BTFErrorKind::InvalidOffset,
                            "Bit offset multiplication overflow",
                        )
                    })?
                    .checked_add(rhs_bit_offset)
                    .ok_or_else(|| {
                        BTFError::new(BTFErrorKind::InvalidOffset, "Bit offset addition overflow")
                    })?;

                Ok(Offset::BitOffsetAndSize(bit_offset, rhs_bit_size))
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

    #[test]
    fn test_u32_addition_overflow() {
        let result = Offset::ByteOffset(u32::MAX).add(1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);

        let result = Offset::ByteOffset(u32::MAX - 1).add(2);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);

        let result = Offset::ByteOffset(u32::MAX).add(0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Offset::ByteOffset(u32::MAX));
    }

    #[test]
    fn test_byte_offset_addition_overflow() {
        let result = Offset::ByteOffset(u32::MAX).add(Offset::ByteOffset(1));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);

        let result = Offset::ByteOffset(0x80000000).add(Offset::ByteOffset(0x80000000));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);

        let result = Offset::ByteOffset(0x7FFFFFFF).add(Offset::ByteOffset(0x7FFFFFFF));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Offset::ByteOffset(0xFFFFFFFE));
    }

    #[test]
    fn test_bit_offset_multiplication_overflow() {
        // The result type of these operations is a BitOffsetAndSize: the operand on the
        // left is multiplied by 8 to convert it to a bit offset.

        // This will overflow when multiplied by 8 for the bit offset conversion
        let unsafe_byte_offset = 0x20000000;

        // Test that the multiplication overflow is caught
        let result = Offset::ByteOffset(unsafe_byte_offset).add(Offset::BitOffsetAndSize(0, 1));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);

        // Test overflow at exact boundary
        let result = Offset::ByteOffset(u32::MAX / 8 + 1).add(Offset::BitOffsetAndSize(0, 1));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);

        // Test valid conversion at maximum safe value
        let result = Offset::ByteOffset(0x1FFFFFFF).add(Offset::BitOffsetAndSize(0, 1));
        assert!(result.is_ok());
    }

    #[test]
    fn test_bit_offset_addition_overflow() {
        // The result type of these operations is a BitOffsetAndSize: the operand on the
        // left is multiplied by 8 to convert it to a bit offset.

        // This will not overflow when multiplied by 8 for the bit offset conversion
        let max_safe_byte_offset = 0x1FFFFFFF;

        // Test that the addition overflow is caught
        let result =
            Offset::ByteOffset(max_safe_byte_offset).add(Offset::BitOffsetAndSize(u32::MAX, 1));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);

        // Test valid bit offset addition
        let result = Offset::ByteOffset(1).add(Offset::BitOffsetAndSize(7, 1));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Offset::BitOffsetAndSize(15, 1));
    }
}
