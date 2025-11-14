/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::{
    btf::{
        parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind,
        Result as BTFResult, Type,
    },
    define_type,
    utils::Reader,
};

/// DeclTag data
#[derive(Debug, Clone)]
struct Data {
    /// Decl tag name
    name: Option<String>,

    /// The type id
    tid: u32,

    /// Component index
    component_index: u32,
}

impl Data {
    /// The size of the extra data
    pub fn size(_type_header: &Header) -> usize {
        4
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        file_header: &FileHeader,
        type_header: &Header,
    ) -> BTFResult<Self> {
        let component_index = reader.u32()?;

        let name = if type_header.name_offset() != 0 {
            Some(parse_string(
                reader,
                file_header,
                type_header.name_offset(),
            )?)
        } else {
            None
        };

        Ok(Self {
            name,
            tid: type_header.size_or_type(),
            component_index,
        })
    }
}

define_type!(DeclTag, Data,
    name: Option<String>,
    tid: u32,
    component_index: u32
);

#[cfg(test)]
mod tests {
    use super::DeclTag;
    use crate::btf::{FileHeader, Header};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_decl_tag() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x10, 0x00, 0x00, 0x00, // type_len
            0x10, 0x00, 0x00, 0x00, // str_off
            0x0A, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x11, // type header: info_flags
            0x04, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x10, 0x00, 0x00, 0x00,
            //
            // String section
            //
            0x00, // mandatory null string
            0x64, 0x65, 0x63, 0x6C, 0x5F, 0x74, 0x61, 0x67, 0x00, // "decl_tag"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let decl_tag = DeclTag::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(*decl_tag.component_index(), 16);
        assert_eq!(decl_tag.name().as_deref(), Some("decl_tag"));
    }
}
