use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Kind,
    Result as BTFResult, Type, TypeHeader,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

define_type!(Volatile);

#[cfg(test)]
mod tests {
    use super::Volatile;
    use crate::btf::{FileHeader, Type, TypeHeader};
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
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let volatile = Volatile::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(volatile.size_or_type(), 3);
    }
}
