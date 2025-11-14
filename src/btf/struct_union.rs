/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::{
    btf::{
        parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind,
        Offset, Result as BTFResult, Type,
    },
    define_type,
    utils::Reader,
};

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
    tid: u32,

    /// The member offset
    offset: Offset,
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
    pub fn tid(&self) -> u32 {
        self.tid
    }

    /// Returns the offset of the member
    pub fn offset(&self) -> Offset {
        self.offset
    }

    /// Creates a new `Member` instance for testing purposes
    #[cfg(test)]
    pub fn create(name_offset: u32, name: Option<String>, tid: u32, offset: Offset) -> Self {
        Self {
            name_offset,
            name,
            tid,
            offset,
        }
    }
}

/// A list of struct or union members
pub type MemberList = Vec<Member>;

/// Struct or union data
#[derive(Debug, Clone)]
struct Data {
    /// The struct or union name
    name: Option<String>,

    /// The total size of the structure, in bytes
    size: usize,

    /// The full member list for this struct or union
    member_list: MemberList,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &Header) -> usize {
        type_header.vlen() * MEMBER_VALUE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        file_header: &FileHeader,
        type_header: &Header,
    ) -> BTFResult<Self> {
        let mut member_list = MemberList::new();

        for _ in 0..type_header.vlen() {
            let name_offset = reader.u32()?;
            let tid = reader.u32()?;
            let raw_offset = reader.u32()?;

            let name = if name_offset != 0 {
                Some(parse_string(reader, file_header, name_offset)?)
            } else {
                None
            };

            let offset = match type_header.kind_flag() {
                false => {
                    if (raw_offset % 8) == 0 {
                        Offset::ByteOffset(raw_offset / 8)
                    } else {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidOffset,
                            "Unaligned bit offset for struct/union member with kind_flag=false",
                        ));
                    }
                }

                true => {
                    let bit_offset = raw_offset & 0xFFFFFF;
                    let bit_size = (raw_offset >> 24) & 0xFF;

                    if bit_size == 0 && (bit_offset % 8) == 0 {
                        Offset::ByteOffset(bit_offset / 8)
                    } else {
                        Offset::BitOffsetAndSize(bit_offset, bit_size)
                    }
                }
            };

            member_list.push(Member {
                name_offset,
                name,
                tid,
                offset,
            });
        }

        let name = if type_header.name_offset() != 0 {
            Some(parse_string(
                reader,
                file_header,
                type_header.name_offset(),
            )?)
        } else {
            None
        };

        Ok(Self {
            name,
            size: type_header.size_or_type() as usize,
            member_list,
        })
    }
}

define_type!(Struct, Data, name: Option<String>, size: usize, member_list: MemberList);
define_type!(Union, Data, name: Option<String>, size: usize, member_list: MemberList);

#[cfg(test)]
mod tests {
    use super::Struct;
    use crate::btf::{FileHeader, Header, Offset};
    use crate::utils::{ReadableBuffer, Reader};
    use crate::Type;

    #[test]
    fn test_standard_struct_union() {
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
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let struct_type = Struct::new(&mut reader, &file_header, type_header).unwrap();

        assert_eq!(*struct_type.size(), 2);
        assert!(!struct_type.header().kind_flag());
        assert_eq!(struct_type.member_list().len(), 2);

        assert_eq!(struct_type.member_list()[0].tid(), 2);
        assert_eq!(struct_type.member_list()[0].offset(), Offset::ByteOffset(1));
        assert_eq!(
            struct_type.member_list()[0].name().as_deref(),
            Some("value1")
        );

        assert_eq!(struct_type.member_list()[1].tid(), 3);
        assert_eq!(struct_type.member_list()[1].offset(), Offset::ByteOffset(1));
        assert_eq!(
            struct_type.member_list()[1].name().as_deref(),
            Some("value2")
        );
    }

    #[test]
    fn test_bitfield_struct_union() {
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
            0x02, 0x00, 0x00, 0x84, // type header: info_flags
            0x02, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // Extra data
            //
            0x01, 0x00, 0x00, 0x00, // member 1: name offset
            0x02, 0x00, 0x00, 0x00, // member 1: type id
            0x08, 0x00, 0x00, 0x00, // member 1: bit offset, bit size (kind flag = 1)
            0x08, 0x00, 0x00, 0x00, // member 2: name offset
            0x03, 0x00, 0x00, 0x00, // member 2: type id
            0x0A, 0x00, 0x00, 0x0B, // member 2: bit offset, bit size (kind flag = 1)
            //
            // String section
            //
            0x00, // mandatory null string
            0x76, 0x61, 0x6C, 0x75, 0x65, 0x31, 0x00, // "value1"
            0x76, 0x61, 0x6C, 0x75, 0x65, 0x32, 0x00, // "value2"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let struct_type = Struct::new(&mut reader, &file_header, type_header).unwrap();

        assert_eq!(*struct_type.size(), 2);
        assert!(struct_type.header().kind_flag());
        assert_eq!(struct_type.member_list().len(), 2);

        // The first member returns a `Offset::ByteOffset` offset because
        // the specified bit offset is a multiple of 8 and the bit size is 0
        assert_eq!(struct_type.member_list()[0].tid(), 2);
        assert_eq!(struct_type.member_list()[0].offset(), Offset::ByteOffset(1));
        assert_eq!(
            struct_type.member_list()[0].name().as_deref(),
            Some("value1")
        );

        assert_eq!(struct_type.member_list()[1].tid(), 3);
        assert_eq!(
            struct_type.member_list()[1].offset(),
            Offset::BitOffsetAndSize(10, 11)
        );
        assert_eq!(
            struct_type.member_list()[1].name().as_deref(),
            Some("value2")
        );
    }
}
