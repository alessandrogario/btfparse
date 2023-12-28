use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Kind,
    Result as BTFResult, Type, TypeHeader,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

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

/// The extra data contained in an DataSec type
#[derive(Debug, Clone)]
pub struct Data {
    variable_list: VariableList,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &TypeHeader) -> usize {
        type_header.vlen() * DATA_SEC_VARIABLE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        _file_header: &FileHeader,
        type_header: &TypeHeader,
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

        Ok(Self { variable_list })
    }

    /// Returns a list of all the variables defined in this data section
    pub fn variable_list(&self) -> VariableList {
        self.variable_list.clone()
    }
}

define_type!(DataSec, Data);

#[cfg(test)]
mod tests {
    use super::DataSec;
    use crate::btf::{FileHeader, Type, TypeHeader};
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
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let data_sec = DataSec::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(data_sec.name().as_deref(), Some("var_name"));
        assert_eq!(data_sec.size_or_type(), 4);

        assert_eq!(data_sec.data().variable_list().len(), 2);
        assert_eq!(data_sec.data().variable_list()[0].var_decl_id, 5);
        assert_eq!(data_sec.data().variable_list()[0].offset, 4);
        assert_eq!(data_sec.data().variable_list()[0].var_size, 8);

        assert_eq!(data_sec.data().variable_list()[1].var_decl_id, 5);
        assert_eq!(data_sec.data().variable_list()[1].offset, 8);
        assert_eq!(data_sec.data().variable_list()[1].var_size, 8);
    }
}
