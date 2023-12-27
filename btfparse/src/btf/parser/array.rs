use crate::btf::parser::{BTFHeader, TypeHeader, TypeKind};
use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};
use crate::utils::Reader;

/// Extra info size
const EXTRA_INFO_SIZE: usize = 12;

/// Represents an array BTF type
pub struct Array {
    /// Element type id
    element_type_id: u32,

    /// Index type id
    index_type_id: u32,

    /// Element count
    element_count: u32,
}

impl Array {
    /// Creates a new array type
    pub fn new(
        reader: &mut Reader,
        btf_header: &BTFHeader,
        type_header: &TypeHeader,
    ) -> BTFResult<Self> {
        let type_section_start = btf_header.hdr_len() + btf_header.type_off();
        let type_section_end = type_section_start + btf_header.type_len();

        if reader.offset() + EXTRA_INFO_SIZE > type_section_end as usize {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeSectionOffset,
                "Invalid type section offset",
            ));
        }

        if type_header.name_offset() != 0 {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid name_offset attribute for array type",
            ));
        }

        if type_header.kind_flag() {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid kind_flag=true attribute for an array type",
            ));
        }

        if type_header.kind() != TypeKind::Array {
            return Err(BTFError::new(
                BTFErrorKind::InvalidBTFKind,
                "Not an array type",
            ));
        }

        if type_header.vlen() != 0 {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid vlen attribute for an array type",
            ));
        }

        if type_header.size_or_type() != 0 {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid size/type attribute for an array type",
            ));
        }

        let element_type_id = reader.u32()?;
        let index_type_id = reader.u32()?;
        let element_count = reader.u32()?;

        Ok(Array {
            element_type_id,
            index_type_id,
            element_count,
        })
    }

    /// Returns the element type id
    pub fn element_type_id(&self) -> u32 {
        self.element_type_id
    }

    /// Returns the index type id
    pub fn index_type_id(&self) -> u32 {
        self.index_type_id
    }

    /// Returns the element count
    pub fn element_count(&self) -> u32 {
        self.element_count
    }
}

#[cfg(test)]
mod tests {
    use super::Array;
    use crate::btf::parser::{BTFHeader, TypeHeader};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_array() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x18, 0x00, 0x00, 0x00, // type_len
            0x18, 0x00, 0x00, 0x00, // str_off
            0x01, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x00, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x03, // type header: info_flags
            0x00, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x05, 0x00, 0x00, 0x00, // array header: element type id
            0x06, 0x00, 0x00, 0x00, // array header: index type id
            0x07, 0x00, 0x00, 0x00, // array header: element count
            //
            // String section
            //
            0x00, // mandatory null string
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let array = Array::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(array.element_type_id(), 5);
        assert_eq!(array.index_type_id(), 6);
        assert_eq!(array.element_count(), 7);
    }
}
