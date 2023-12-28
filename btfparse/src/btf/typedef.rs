use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Kind,
    Result as BTFResult, Type, TypeHeader,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

define_type!(Typedef);

#[cfg(test)]
mod tests {
    use super::Typedef;
    use crate::btf::Type;
    use crate::btf::{FileHeader, TypeHeader};
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
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let typedef_type = Typedef::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(typedef_type.name().as_deref(), Some("void*"));
        assert_eq!(typedef_type.size_or_type(), 0);
    }
}
