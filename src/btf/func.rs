/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind,
    Result as BTFResult, Type,
};
use crate::define_type;
use crate::utils::Reader;

/// Func data
#[derive(Debug, Clone)]
struct Data {
    /// The function name
    name: Option<String>,

    /// Prototype type id
    prototype_tid: u32,
}

impl Data {
    /// The size of the extra data
    pub fn size(_type_header: &Header) -> usize {
        0
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        file_header: &FileHeader,
        type_header: &Header,
    ) -> BTFResult<Self> {
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
            prototype_tid: type_header.size_or_type(),
        })
    }
}

define_type!(Func, Data, name: Option<String>, prototype_tid: u32);

#[cfg(test)]
mod tests {
    use super::Func;
    use crate::btf::{FileHeader, Header};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_fwd() {
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
            0x0C, 0x00, 0x00, 0x00, // str_off
            0x06, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x0C, // type header: info_flags
            0x03, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // String section
            //
            0x00, // mandatory null string
            0x65, 0x78, 0x69, 0x74, 0x00, // "exit"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let func = Func::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(func.name().as_deref(), Some("exit"));
        assert_eq!(*func.prototype_tid(), 3);
    }
}
