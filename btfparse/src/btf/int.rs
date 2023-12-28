use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Kind,
    Result as BTFResult, Type, TypeHeader,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

/// The extra data contained in an int type
#[derive(Debug, Clone, Copy)]
pub struct Data {
    /// Whether the integer is signed
    signed: bool,

    /// Whether the integer is a char
    char: bool,

    /// Whether the integer is a boolean
    boolean: bool,

    /// The offset, in bits, of the integer. Used for bitfields
    offset: usize,

    /// The number of bits in the integer. Used for bitfields
    bits: usize,
}

impl Data {
    /// The size of the extra data
    pub fn size(_type_header: &TypeHeader) -> usize {
        4
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        _file_header: &FileHeader,
        _type_header: &TypeHeader,
    ) -> BTFResult<Self> {
        let extra_info = reader.u32()?;
        let encoding = (extra_info & 0x0F000000) >> 24;
        let offset = ((extra_info & 0x00FF0000) >> 16) as usize;
        let bits = (extra_info & 0x000000FF) as usize;

        Ok(Self {
            signed: (encoding & 1) != 0,
            char: (encoding & 2) != 0,
            boolean: (encoding & 4) != 0,
            offset,
            bits,
        })
    }

    /// Returns whether the integer is signed
    pub fn signed(&self) -> bool {
        self.signed
    }

    /// Returns whether the integer is a char
    pub fn char(&self) -> bool {
        self.char
    }

    /// Returns whether the integer is a boolean
    pub fn boolean(&self) -> bool {
        self.boolean
    }

    /// Returns the offset, in bits, of the integer. Used for bitfields
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the number of bits in the integer. Used for bitfields
    pub fn bits(&self) -> usize {
        self.bits
    }
}

define_type!(Int, Data);

#[cfg(test)]
mod tests {
    use super::Int;
    use crate::btf::{FileHeader, Type, TypeHeader};
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
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let int_type = Int::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(int_type.name().as_deref(), Some("unsigned int"));
        assert_eq!(int_type.size_or_type(), 4);
        assert!(!int_type.data().signed());
        assert!(!int_type.data().boolean());
        assert!(!int_type.data().char());
        assert_eq!(int_type.data().offset(), 8);
        assert_eq!(int_type.data().bits(), 16);
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
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let int_type = Int::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(int_type.name().as_deref(), Some("char"));
        assert_eq!(int_type.size_or_type(), 1);
        assert!(!int_type.data().signed());
        assert!(!int_type.data().boolean());
        assert!(int_type.data().char());
        assert_eq!(int_type.data().offset(), 0);
        assert_eq!(int_type.data().bits(), 8);
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
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let int_type = Int::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(int_type.name().as_deref(), Some("int"));
        assert_eq!(int_type.size_or_type(), 1);
        assert!(int_type.data().signed());
        assert!(!int_type.data().boolean());
        assert!(!int_type.data().char());
        assert_eq!(int_type.data().offset(), 0);
        assert_eq!(int_type.data().bits(), 8);
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
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let int_type = Int::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(int_type.name().as_deref(), Some("bool"));
        assert_eq!(int_type.size_or_type(), 1);
        assert!(!int_type.data().signed());
        assert!(int_type.data().boolean());
        assert!(!int_type.data().char());
        assert_eq!(int_type.data().offset(), 0);
        assert_eq!(int_type.data().bits(), 8);
    }
}
