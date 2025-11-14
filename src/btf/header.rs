/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::btf::{
    Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Kind, Result as BTFResult,
};
use crate::utils::Reader;

/// Type header size
const TYPE_HEADER_SIZE: usize = 12;

/// Common type header
#[derive(Debug, Clone, Copy)]
pub struct Header {
    /// Type kind
    kind: Kind,

    /// Offset of the type name
    name_offset: u32,

    /// Type-related size, for example the member count in a struct
    vlen: usize,

    /// Type-related flag, used by struct, union, fwd, enum and enum64
    kind_flag: bool,

    /// Depending on the type, it's either a size or a type id
    size_or_type: u32,
}

impl Header {
    /// Creates a new `TypeHeader` instance
    pub fn new(reader: &mut Reader, btf_header: &FileHeader) -> BTFResult<Header> {
        let type_section_start = btf_header
            .hdr_len()
            .checked_add(btf_header.type_off())
            .ok_or_else(|| {
                BTFError::new(
                    BTFErrorKind::InvalidTypeSectionOffset,
                    "Type section start offset overflow",
                )
            })?;

        let type_section_end = type_section_start
            .checked_add(btf_header.type_len())
            .ok_or_else(|| {
                BTFError::new(
                    BTFErrorKind::InvalidTypeSectionOffset,
                    "Type section end offset overflow",
                )
            })?;

        if reader.offset() + TYPE_HEADER_SIZE > type_section_end as usize {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeSectionOffset,
                "Invalid type section offset",
            ));
        }

        let name_offset = reader.u32()?;
        let info_flags = reader.u32()?;
        let vlen = (info_flags & 0xFFFF) as usize;
        let kind = Kind::new((info_flags & 0x1F000000) >> 24)?;
        let kind_flag = (info_flags & 0x80000000) != 0;
        let size_or_type = reader.u32()?;

        Ok(Header {
            kind,
            name_offset,
            vlen,
            kind_flag,
            size_or_type,
        })
    }

    /// Returns the raw `kind` value
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Returns the offset of the type name
    pub fn name_offset(&self) -> u32 {
        self.name_offset
    }

    /// Returns the raw `vlen` value
    pub fn vlen(&self) -> usize {
        self.vlen
    }

    /// Returns the raw `kind_flag` value
    pub fn kind_flag(&self) -> bool {
        self.kind_flag
    }

    /// Returns the raw `size_or_type` value
    pub fn size_or_type(&self) -> u32 {
        self.size_or_type
    }

    /// Creates a new `Header` instance for testing purposes
    #[cfg(test)]
    pub fn create(
        kind: Kind,
        name_offset: u32,
        vlen: usize,
        kind_flag: bool,
        size_or_type: u32,
    ) -> Header {
        Header {
            kind,
            name_offset,
            vlen,
            kind_flag,
            size_or_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Header;
    use crate::btf::{FileHeader, Kind};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_type_header() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x0C, 0x00, 0x00, 0x00, // type_len
            0x00, 0x00, 0x00, 0x00, // str_off
            0x00, 0x00, 0x00, 0x00, // str_len
            //
            // Type header
            //
            0x01, 0x00, 0x00, 0x00, // name_offset
            0xFF, 0x00, 0x00, 0x81, // info_flags
            0x03, 0x00, 0x00, 0x00, // size_or_type
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &btf_header).unwrap();

        assert_eq!(type_header.kind(), Kind::Int);
        assert_eq!(type_header.name_offset(), 1);
        assert_eq!(type_header.vlen(), 255);
        assert!(type_header.kind_flag());
        assert_eq!(type_header.size_or_type(), 3);
    }

    #[test]
    fn test_type_section_start_overflow() {
        // Test overflow when calculating type section start (hdr_len + type_off)
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0xFF, 0xFF, 0xFF, 0x7F, // hdr_len
            0xFF, 0xFF, 0xFF, 0x7F, // type_off
            0x0C, 0x00, 0x00, 0x00, // type_len
            0x00, 0x00, 0x00, 0x00, // str_off
            0x00, 0x00, 0x00, 0x00, // str_len
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = FileHeader::new(&mut reader).unwrap();

        let result = Header::new(&mut reader, &btf_header);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            crate::btf::ErrorKind::InvalidTypeSectionOffset
        );
    }

    #[test]
    fn test_type_section_end_overflow() {
        // Test overflow when calculating type section end (start + type_len)
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0xFF, // type_off
            0xFF, 0xFF, 0xFF, 0x01, // type_len
            0x00, 0x00, 0x00, 0x00, // str_off
            0x00, 0x00, 0x00, 0x00, // str_len
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = FileHeader::new(&mut reader).unwrap();

        let result = Header::new(&mut reader, &btf_header);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            crate::btf::ErrorKind::InvalidTypeSectionOffset
        );
    }
}
