use crate::btf::parser::{BTFHeader, TypeKind};
use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};
use crate::utils::Reader;

/// Type header size
const TYPE_HEADER_SIZE: usize = 12;

/// Common type header
pub struct TypeHeader {
    /// Type kind
    kind: TypeKind,

    /// Offset of the type name
    name_offset: u32,

    /// Type-related size, for example the member count in a struct
    vlen: usize,

    /// Type-related flag, used by struct, union, fwd, enum and enum64
    kind_flag: bool,

    /// Depending on the type, it's either a size or a type id
    size_or_type: u32,
}

impl TypeHeader {
    /// Creates a new `TypeHeader` instance
    pub fn new(reader: &mut Reader, btf_header: &BTFHeader) -> BTFResult<TypeHeader> {
        let type_section_start = btf_header.hdr_len() + btf_header.type_off();
        let type_section_end = type_section_start + btf_header.type_len();

        if reader.offset() + TYPE_HEADER_SIZE > type_section_end as usize {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeSectionOffset,
                "Invalid type section offset",
            ));
        }

        let name_offset = reader.u32()?;
        let info_flags = reader.u32()?;
        let vlen = (info_flags & 0xFFFF) as usize;
        let type_kind = TypeKind::new((info_flags & 0x1F000000) >> 24)?;
        let kind_flag = (info_flags & 0x80000000) != 0;
        let size_or_type = reader.u32()?;

        Ok(TypeHeader {
            kind: type_kind,
            name_offset,
            vlen,
            kind_flag,
            size_or_type,
        })
    }

    /// Returns the raw `kind` value
    pub fn kind(&self) -> TypeKind {
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
}

#[cfg(test)]
mod tests {
    use super::TypeHeader;
    use crate::btf::parser::{BTFHeader, TypeKind};
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
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();

        assert_eq!(type_header.kind(), TypeKind::Int);
        assert_eq!(type_header.name_offset(), 1);
        assert_eq!(type_header.vlen(), 255);
        assert!(type_header.kind_flag());
        assert_eq!(type_header.size_or_type(), 3);
    }
}
