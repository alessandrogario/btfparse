use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind,
    Result as BTFResult, Type,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

/// The linkage type of the var
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkageType {
    Static,
    Global,
}

/// Var data
#[derive(Debug, Clone)]
pub struct Data {
    /// The var name
    name: Option<String>,

    /// The type id of the var
    type_id: u32,

    /// The raw linkage field from the type section
    linkage: u32,

    /// Linkage type
    linkage_type: LinkageType,
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
        let linkage = reader.u32()?;
        let linkage_type = match linkage {
            0 => LinkageType::Static,
            _ => LinkageType::Global,
        };

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
            type_id: type_header.size_or_type(),
            linkage,
            linkage_type,
        })
    }
}

define_type!(Var, Data,
    name: Option<String>,
    type_id: u32,
    linkage: u32,
    linkage_type: LinkageType
);

#[cfg(test)]
mod tests {
    use super::{LinkageType, Var};
    use crate::btf::{FileHeader, Header};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_var() {
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
            0x0C, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x0E, // type header: info_flags
            0x05, 0x00, 0x00, 0x00, // type header: size_or_type
            // Linkage type
            0x01, 0x00, 0x00, 0x00,
            //
            // String section
            //
            0x00, // mandatory null string
            0x73, 0x74, 0x61, 0x74, 0x69, 0x63, 0x5F, 0x76, 0x61, 0x72, 0x00, // "static_var"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let var_type = Var::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(var_type.name().as_deref(), Some("static_var"));
        assert_eq!(*var_type.type_id(), 5);
        assert_eq!(*var_type.linkage_type(), LinkageType::Global);
        assert_eq!(*var_type.linkage(), 0x01);
    }
}
