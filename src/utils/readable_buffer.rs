/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Readable, Result as BTFResult};

pub struct ReadableBuffer<'a> {
    buffer: &'a [u8],
}

impl<'a> ReadableBuffer<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer }
    }
}

impl<'a> Readable for ReadableBuffer<'a> {
    fn read(&self, offset: u64, buffer: &mut [u8]) -> BTFResult<()> {
        let source_start_offset = offset as usize;
        if source_start_offset == self.buffer.len() {
            return Err(BTFError::new(
                BTFErrorKind::EOF,
                "There are no bytes left to read",
            ));
        }

        let source_end_offset = source_start_offset
            .checked_add(buffer.len())
            .ok_or_else(|| {
                BTFError::new(
                    BTFErrorKind::InvalidOffset,
                    "Buffer offset addition overflow",
                )
            })?;

        match source_end_offset.cmp(&self.buffer.len()) {
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                let source_slice = &self.buffer[source_start_offset..source_end_offset];
                buffer.copy_from_slice(source_slice);

                Ok(())
            }

            std::cmp::Ordering::Greater => Err(BTFError::new(
                BTFErrorKind::InvalidOffset,
                "There are not enough bytes left to complete the read request",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_offset_overflow() {
        // Test overflow when calculating source_end_offset (source_start_offset + buffer.len())
        let data = vec![0u8; 100];
        let readable_buffer = ReadableBuffer::new(&data);

        let mut buffer = vec![0u8; 100];
        let result = readable_buffer.read(usize::MAX as u64 - 50, &mut buffer);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);
    }

    #[test]
    fn test_valid_read_at_boundary() {
        let data = vec![0x41, 0x42, 0x43, 0x44];
        let readable_buffer = ReadableBuffer::new(&data);

        let mut buffer = vec![0u8; 4];
        let result = readable_buffer.read(0, &mut buffer);
        assert!(result.is_ok());
        assert_eq!(buffer, vec![0x41, 0x42, 0x43, 0x44]);

        let mut buffer = vec![0u8; 1];
        let result = readable_buffer.read(3, &mut buffer);
        assert!(result.is_ok());
        assert_eq!(buffer, vec![0x44]);
    }

    #[test]
    fn test_eof_error() {
        let data = vec![0x41, 0x42, 0x43, 0x44];
        let readable_buffer = ReadableBuffer::new(&data);

        let mut buffer = vec![0u8; 1];
        let result = readable_buffer.read(4, &mut buffer);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::EOF);
    }

    #[test]
    fn test_invalid_offset_beyond_buffer() {
        let data = vec![0x41, 0x42, 0x43, 0x44];
        let readable_buffer = ReadableBuffer::new(&data);

        let mut buffer = vec![0u8; 2];
        let result = readable_buffer.read(3, &mut buffer);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidOffset);
    }
}
