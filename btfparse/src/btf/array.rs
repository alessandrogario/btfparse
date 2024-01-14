use crate::btf::{
    Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind, Result as BTFResult,
    Type,
};
use crate::define_type;
use crate::utils::Reader;

/// The size of the extra data
const ENUM_VALUE_SIZE: usize = 12;

/// Array data
#[derive(Debug, Clone, Copy)]
struct Data {
    /// The element type id
    element_type_id: u32,

    /// The index type id
    index_type_id: u32,

    /// The number of elements in the array
    element_count: u32,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &Header) -> usize {
        type_header.vlen() * ENUM_VALUE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        _file_header: &FileHeader,
        _type_header: &Header,
    ) -> BTFResult<Self> {
        let element_type_id = reader.u32()?;
        let index_type_id = reader.u32()?;
        let element_count = reader.u32()?;

        Ok(Data {
            element_type_id,
            index_type_id,
            element_count,
        })
    }
}

define_type!(Array, Data,
    element_type_id: u32,
    index_type_id: u32,
    element_count: u32
);

#[cfg(test)]
mod tests {
    use super::Array;
    use crate::btf::{FileHeader, Header};
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
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let array = Array::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(*array.element_type_id(), 5);
        assert_eq!(*array.index_type_id(), 6);
        assert_eq!(*array.element_count(), 7);
    }
}
