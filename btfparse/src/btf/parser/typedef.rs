use crate::btf::parser::{parse_string, BTFHeader, TypeHeader, TypeKind};
use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};
use crate::utils::Reader;

/// Represents a Typedef BTF type
pub struct Typedef {
    /// The name of the type
    name: String,

    /// The id of the type used by this typedef
    type_id: u32,
}

impl Typedef {
    /// Creates a new typedef type
    pub fn new(
        reader: &mut Reader,
        btf_header: &BTFHeader,
        type_header: &TypeHeader,
    ) -> BTFResult<Self> {
        if type_header.kind() != TypeKind::Typedef {
            return Err(BTFError::new(
                BTFErrorKind::InvalidBTFKind,
                "Not a typedef type",
            ));
        }

        if type_header.kind_flag() {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid kind_flag=true attribute for typedef type",
            ));
        }

        if type_header.vlen() != 0 {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid vlen attribute for typedef type",
            ));
        }

        let name = parse_string(reader, btf_header, type_header.name_offset())?;
        let type_id = type_header.size_or_type();

        Ok(Typedef { name, type_id })
    }

    /// Returns the type name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the type id
    pub fn type_id(&self) -> u32 {
        self.type_id
    }
}

#[cfg(test)]
mod tests {
    use super::Typedef;
    use crate::btf::parser::{BTFHeader, TypeHeader};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_typedef() {
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
            0x07, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x08, // type header: info_flags
            0x00, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // String section
            //
            0x00, // mandatory null string
            0x76, 0x6F, 0x69, 0x64, 0x2A, 0x00, // "void*"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let typedef_type = Typedef::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(typedef_type.name(), "void*");
        assert_eq!(typedef_type.type_id(), 0);
    }
}
