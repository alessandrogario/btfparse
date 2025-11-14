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

/// The size of a single variable decl defined in this data section
const DATA_SEC_VARIABLE_SIZE: usize = 12;

/// A single variable decl defined in this data section
#[derive(Debug, Clone, Copy)]
pub struct Variable {
    /// The type id of the Var decl
    pub var_decl_id: u32,

    /// The offset of the Var decl inside the data section
    pub offset: u32,

    /// The size of the Var decl
    pub var_size: u32,
}

/// A list of variables
pub type VariableList = Vec<Variable>;

/// DataSec date
#[derive(Debug, Clone)]
struct Data {
    /// The data sec name
    name: Option<String>,

    /// The data sec size
    size: usize,

    /// A list of variables defined in this data section
    variable_list: VariableList,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &Header) -> usize {
        type_header.vlen() * DATA_SEC_VARIABLE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        file_header: &FileHeader,
        type_header: &Header,
    ) -> BTFResult<Self> {
        let mut variable_list = VariableList::new();

        for _ in 0..type_header.vlen() {
            let variable = Variable {
                var_decl_id: reader.u32()?,
                offset: reader.u32()?,
                var_size: reader.u32()?,
            };

            variable_list.push(variable);
        }

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
            size: type_header.size_or_type() as usize,
            variable_list,
        })
    }
}

define_type!(DataSec, Data,
    name: Option<String>,
    size: usize,
    variable_list: VariableList
);

#[cfg(test)]
mod tests {
    use super::DataSec;
    use crate::btf::{FileHeader, Header};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_data_sec() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x24, 0x00, 0x00, 0x00, // type_len
            0x24, 0x00, 0x00, 0x00, // str_off
            0x0A, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x02, 0x00, 0x00, 0x0F, // type header: info_flags
            0x04, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x05, 0x00, 0x00, 0x00, // datasec info 1: var decl id
            0x04, 0x00, 0x00, 0x00, // datasec info 1: var decl offset
            0x08, 0x00, 0x00, 0x00, // datasec info 1: var decl size
            0x05, 0x00, 0x00, 0x00, // datasec info 2: var decl id
            0x08, 0x00, 0x00, 0x00, // datasec info 2: var decl offset
            0x08, 0x00, 0x00, 0x00, // datasec info 2: var decl size
            //
            // String section
            //
            0x00, // mandatory null string
            0x76, 0x61, 0x72, 0x5F, 0x6E, 0x61, 0x6D, 0x65, 0x00, // "var_name"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let data_sec = DataSec::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(data_sec.name().as_deref(), Some("var_name"));
        assert_eq!(*data_sec.size(), 4);

        assert_eq!(data_sec.variable_list().len(), 2);
        assert_eq!(data_sec.variable_list()[0].var_decl_id, 5);
        assert_eq!(data_sec.variable_list()[0].offset, 4);
        assert_eq!(data_sec.variable_list()[0].var_size, 8);

        assert_eq!(data_sec.variable_list()[1].var_decl_id, 5);
        assert_eq!(data_sec.variable_list()[1].offset, 8);
        assert_eq!(data_sec.variable_list()[1].var_size, 8);
    }
}
