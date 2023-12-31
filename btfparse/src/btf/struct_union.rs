use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Kind,
    Result as BTFResult, Type, TypeHeader,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

/// The size required to hold the extra data for a single member
const MEMBER_VALUE_SIZE: usize = 12;

/// A single member for a struct or union
#[derive(Debug, Clone)]
pub struct Member {
    /// The raw string section offset
    name_offset: u32,

    /// The member name
    name: Option<String>,

    /// The member type id
    type_id: u32,

    /// The member offset
    offset: u32,
}

impl Member {
    /// Returns the raw string section offset
    pub fn name_offset(&self) -> u32 {
        self.name_offset
    }

    /// Returns the member name
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Returns the type id of the member
    pub fn type_id(&self) -> u32 {
        self.type_id
    }

    /// Returns the offset of the member
    pub fn offset(&self) -> u32 {
        self.offset
    }
}

/// A list of struct or union members
pub type MemberList = Vec<Member>;

/// The extra data contained in an int type
#[derive(Debug, Clone)]
pub struct Data {
    /// The full member list for this struct or union
    member_list: MemberList,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &TypeHeader) -> usize {
        type_header.vlen() * MEMBER_VALUE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        file_header: &FileHeader,
        type_header: &TypeHeader,
    ) -> BTFResult<Self> {
        let mut member_list = MemberList::new();

        for _ in 0..type_header.vlen() {
            let name_offset = reader.u32()?;
            let type_id = reader.u32()?;
            let offset = reader.u32()?;

            let name = if name_offset != 0 {
                Some(parse_string(reader, file_header, name_offset)?)
            } else {
                None
            };

            member_list.push(Member {
                name_offset,
                name,
                type_id,
                offset,
            });
        }

        Ok(Self { member_list })
    }

    /// Returns full member list
    pub fn member_list(&self) -> MemberList {
        self.member_list.clone()
    }
}

define_type!(Struct, Data);
define_type!(Union, Data);

#[cfg(test)]
mod tests {
    use super::{Struct, Union};
    use crate::btf::{FileHeader, Type, TypeHeader};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_struct() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x24, 0x00, 0x00, 0x00, // type_len
            0x24, 0x00, 0x00, 0x00, // str_off
            0x0F, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x00, 0x00, 0x00, 0x00, // type header: name_offset
            0x02, 0x00, 0x00, 0x04, // type header: info_flags
            0x02, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // Extra data
            //
            0x01, 0x00, 0x00, 0x00, // member 1: name offset
            0x02, 0x00, 0x00, 0x00, // member 1: type id
            0x08, 0x00, 0x00, 0x00, // member 1: offset
            0x08, 0x00, 0x00, 0x00, // member 2: name offset
            0x03, 0x00, 0x00, 0x00, // member 2: type id
            0x08, 0x00, 0x00, 0x00, // member 2: offset
            //
            // String section
            //
            0x00, // mandatory null string
            0x76, 0x61, 0x6C, 0x75, 0x65, 0x31, 0x00, // "value1"
            0x76, 0x61, 0x6C, 0x75, 0x65, 0x32, 0x00, // "value2"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let func_proto_type = Struct::new(&mut reader, &file_header, type_header).unwrap();

        assert_eq!(func_proto_type.size_or_type(), 2);
        assert!(!func_proto_type.kind_flag());
        assert_eq!(func_proto_type.data().member_list().len(), 2);

        assert_eq!(func_proto_type.data().member_list()[0].type_id(), 2);
        assert_eq!(func_proto_type.data().member_list()[0].offset(), 8);
        assert_eq!(
            func_proto_type.data().member_list()[0].name().as_deref(),
            Some("value1")
        );

        assert_eq!(func_proto_type.data().member_list()[1].type_id(), 3);
        assert_eq!(func_proto_type.data().member_list()[1].offset(), 8);
        assert_eq!(
            func_proto_type.data().member_list()[1].name().as_deref(),
            Some("value2")
        );
    }

    #[test]
    fn test_union() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x24, 0x00, 0x00, 0x00, // type_len
            0x24, 0x00, 0x00, 0x00, // str_off
            0x0F, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x00, 0x00, 0x00, 0x00, // type header: name_offset
            0x02, 0x00, 0x00, 0x05, // type header: info_flags
            0x02, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // Extra data
            //
            0x01, 0x00, 0x00, 0x00, // member 1: name offset
            0x02, 0x00, 0x00, 0x00, // member 1: type id
            0x08, 0x00, 0x00, 0x00, // member 1: offset
            0x08, 0x00, 0x00, 0x00, // member 2: name offset
            0x03, 0x00, 0x00, 0x00, // member 2: type id
            0x08, 0x00, 0x00, 0x00, // member 2: offset
            //
            // String section
            //
            0x00, // mandatory null string
            0x76, 0x61, 0x6C, 0x75, 0x65, 0x31, 0x00, // "value1"
            0x76, 0x61, 0x6C, 0x75, 0x65, 0x32, 0x00, // "value2"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &file_header).unwrap();
        let func_proto_type = Union::new(&mut reader, &file_header, type_header).unwrap();

        assert_eq!(func_proto_type.size_or_type(), 2);
        assert!(!func_proto_type.kind_flag());
        assert_eq!(func_proto_type.data().member_list().len(), 2);

        assert_eq!(func_proto_type.data().member_list()[0].type_id(), 2);
        assert_eq!(func_proto_type.data().member_list()[0].offset(), 8);
        assert_eq!(
            func_proto_type.data().member_list()[0].name().as_deref(),
            Some("value1")
        );

        assert_eq!(func_proto_type.data().member_list()[1].type_id(), 3);
        assert_eq!(func_proto_type.data().member_list()[1].offset(), 8);
        assert_eq!(
            func_proto_type.data().member_list()[1].name().as_deref(),
            Some("value2")
        );
    }
}
