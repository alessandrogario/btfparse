use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Kind,
    Result as BTFResult, Type, TypeHeader,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

/// The size of the extra data
const ENUM_VALUE_SIZE: usize = 12;

/// The extra data contained in an array type
#[derive(Debug, Clone, Copy)]
pub struct Data {
    /// The element type id
    element_type_id: u32,

    /// The index type id
    index_type_id: u32,

    /// The number of elements in the array
    element_count: u32,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &TypeHeader) -> usize {
        type_header.vlen() * ENUM_VALUE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        _file_header: &FileHeader,
        _type_header: &TypeHeader,
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

define_type!(Array, Data);

#[cfg(test)]
mod tests {
    use super::Array;
    use crate::btf::{FileHeader, TypeHeader};
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
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let array = Array::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(array.data().element_type_id(), 5);
        assert_eq!(array.data().index_type_id(), 6);
        assert_eq!(array.data().element_count(), 7);
    }
}
