use crate::btf::{
    Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind, Result as BTFResult,
    Type,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

/// Ptr data
#[derive(Debug, Clone)]
pub struct Data {
    /// The pointee type
    type_id: u32,
}

impl Data {
    /// The size of the extra data
    pub fn size(_type_header: &Header) -> usize {
        0
    }

    /// Creates a new `Data` object
    pub fn new(
        _reader: &mut Reader,
        _file_header: &FileHeader,
        type_header: &Header,
    ) -> BTFResult<Self> {
        Ok(Self {
            type_id: type_header.size_or_type(),
        })
    }
}

define_type!(Ptr, Data, type_id: u32);

#[cfg(test)]
mod tests {
    use super::Ptr;
    use crate::btf::{FileHeader, Header};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_ptr() {
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
            0x00, 0x00, 0x00, 0x02, // type header: info_flags
            0x03, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // String section
            //
            0x00, // mandatory null string
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let ptr_type = Ptr::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(*ptr_type.type_id(), 3);
    }
}
