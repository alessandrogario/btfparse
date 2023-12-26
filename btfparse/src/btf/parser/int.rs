use crate::btf::parser::{parse_string, BTFHeader, TypeHeader, TypeKind};
use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};
use crate::utils::Reader;

/// Extra info size
const EXTRA_INFO_SIZE: usize = 4;

/// BTF int type encoding options
enum EncodingOption {
    None,
    Char,
    Bool,
}

/// Represents an int BTF type
pub struct Int {
    /// The name of the type
    name: String,

    /// The size of the type
    size: usize,

    /// Whether the type is signed or not
    signed: bool,

    /// The encoding option for the type
    encoding_option: EncodingOption,

    /// The offset of the type
    offset: usize,

    /// The number of bits of the type
    bits: usize,
}

impl Int {
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

        if type_header.vlen() != 0 {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid vlen attribute for an int type",
            ));
        }

        if type_header.kind() != TypeKind::Int {
            return Err(BTFError::new(
                BTFErrorKind::InvalidBTFKind,
                "Not an integer type",
            ));
        }

        if type_header.kind_flag() {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeHeaderAttribute,
                "Invalid kind_flag=true attribute for int type",
            ));
        }

        let extra_info = reader.u32()?;
        let encoding = (extra_info & 0x0F000000) >> 24;
        let offset = ((extra_info & 0x00FF0000) >> 16) as usize;
        let bits = (extra_info & 0x000000FF) as usize;

        let name = parse_string(reader, btf_header, type_header.name_offset())?;
        let size = type_header.size_or_type() as usize;

        let signed = (encoding & 1) != 0;
        let char = (encoding & 2) != 0;
        let boolean = (encoding & 4) != 0;

        let encoding_option = match (signed, char, boolean) {
            (false, false, false) => EncodingOption::None,
            (true, false, false) => EncodingOption::None,
            (false, true, false) => EncodingOption::Char,
            (false, false, true) => EncodingOption::Bool,

            _ => {
                return Err(BTFError::new(
                    BTFErrorKind::InvalidTypeHeaderAttribute,
                    "Invalid encoding attribute for int type",
                ));
            }
        };

        Ok(Int {
            name,
            size,
            signed,
            encoding_option,
            offset,
            bits,
        })
    }

    /// Returns the type name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the type size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns whether the type is signed or not
    pub fn signed(&self) -> bool {
        self.signed
    }

    /// Returns true if the encoding options mark this type as a boolean
    pub fn boolean(&self) -> bool {
        matches!(self.encoding_option, EncodingOption::Bool)
    }

    /// Returns true if the encoding options mark this type as a char
    pub fn char(&self) -> bool {
        matches!(self.encoding_option, EncodingOption::Char)
    }

    /// Returns the type offset (i.e.: bitfields)
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the number of bits of the type
    pub fn bits(&self) -> usize {
        self.bits
    }
}

#[cfg(test)]
mod tests {
    use super::Int;
    use crate::btf::parser::{BTFHeader, TypeHeader};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_int_no_encoding_options() {
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
            0x0E, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x01, // type header: info_flags
            0x04, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x10, 0x00, 0x08, 0x00,
            //
            // String section
            //
            0x00, // mandatory null string
            0x75, 0x6E, 0x73, 0x69, 0x67, 0x6E, 0x65, 0x64, 0x20, 0x69, 0x6E, 0x74,
            0x00, // "unsigned int"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let int_type = Int::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(int_type.name(), "unsigned int");
        assert_eq!(int_type.size(), 4);
        assert!(!int_type.signed());
        assert!(!int_type.boolean());
        assert!(!int_type.char());
        assert_eq!(int_type.offset(), 8);
        assert_eq!(int_type.bits(), 16);
    }

    #[test]
    fn test_int_with_char_encoding_option() {
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
            0x06, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x01, // type header: info_flags
            0x01, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x08, 0x00, 0x00, 0x02,
            //
            // String section
            //
            0x00, // mandatory null string
            0x63, 0x68, 0x61, 0x72, 0x00, // char
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let int_type = Int::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(int_type.name(), "char");
        assert_eq!(int_type.size(), 1);
        assert!(!int_type.signed());
        assert!(!int_type.boolean());
        assert!(int_type.char());
        assert_eq!(int_type.offset(), 0);
        assert_eq!(int_type.bits(), 8);
    }

    #[test]
    fn test_int_with_signed_encoding_option() {
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
            0x05, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x01, // type header: info_flags
            0x01, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x08, 0x00, 0x00, 0x01,
            //
            // String section
            //
            0x00, // mandatory null string
            0x69, 0x6E, 0x74, 0x00, // "int"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let int_type = Int::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(int_type.name(), "int");
        assert_eq!(int_type.size(), 1);
        assert!(int_type.signed());
        assert!(!int_type.boolean());
        assert!(!int_type.char());
        assert_eq!(int_type.offset(), 0);
        assert_eq!(int_type.bits(), 8);
    }

    #[test]
    fn test_int_with_bool_encoding_option() {
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
            0x06, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x01, // type header: info_flags
            0x01, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x08, 0x00, 0x00, 0x04,
            //
            // String section
            //
            0x00, // mandatory null string
            0x62, 0x6F, 0x6F, 0x6C, 0x00, // "bool"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let int_type = Int::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(int_type.name(), "bool");
        assert_eq!(int_type.size(), 1);
        assert!(!int_type.signed());
        assert!(int_type.boolean());
        assert!(!int_type.char());
        assert_eq!(int_type.offset(), 0);
        assert_eq!(int_type.bits(), 8);
    }

    #[test]
    fn test_int_with_invalid_encoding_options() {
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
            0x06, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x00, 0x00, 0x00, 0x01, // type header: info_flags
            0x01, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x08, 0x00, 0x00, 0x0F,
            //
            // String section
            //
            0x00, // mandatory null string
            0x62, 0x6F, 0x6F, 0x6C, 0x00, // "bool"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        assert!(Int::new(&mut reader, &btf_header, &type_header).is_err());
    }
}
