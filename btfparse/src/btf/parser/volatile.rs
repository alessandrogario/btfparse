use crate::btf::parser::{TypeHeader, TypeKind};
use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};

/// Represents a Volatile BTF type
pub struct Volatile {
    /// The id of the type used by this volatile
    type_id: u32,
}

impl Volatile {
    /// Creates a new volatile type
    pub fn new(type_header: &TypeHeader) -> BTFResult<Self> {
        if type_header.kind() != TypeKind::Volatile {
            return Err(BTFError::new(
                BTFErrorKind::InvalidBTFKind,
                "Not a volatile type",
            ));
        }

        if type_header.kind_flag() {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid kind_flag=true attribute for volatile type",
            ));
        }

        if type_header.vlen() != 0 {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid vlen attribute for volatile type",
            ));
        }

        if type_header.name_offset() != 0 {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid name_offset attribute for volatile type",
            ));
        }

        let type_id = type_header.size_or_type();
        Ok(Volatile { type_id })
    }

    /// Returns the type id
    pub fn type_id(&self) -> u32 {
        self.type_id
    }
}

#[cfg(test)]
mod tests {
    use super::Volatile;
    use crate::btf::parser::{BTFHeader, TypeHeader};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_volatile() {
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
            0x01, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x00, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x09, // type header: info_flags
            0x03, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // String section
            //
            0x00, // mandatory null string
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let volatile = Volatile::new(&type_header).unwrap();
        assert_eq!(volatile.type_id(), 3);
    }
}
