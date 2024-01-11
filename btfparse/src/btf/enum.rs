use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind,
    Result as BTFResult, Type,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

/// The size of the extra data (one per enum value)
const ENUM_VALUE_SIZE: usize = 8;

/// Represents an enum value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerValue {
    /// The signed value
    Signed(i32),

    /// The unsigned value
    Unsigned(u32),
}

/// Represents a single enum value
#[derive(Debug, Clone)]
pub struct NamedValue {
    /// The name of the value
    pub name: String,

    /// The signed value
    pub value: IntegerValue,
}

/// Represents a list of enum values
pub type NamedValueList = Vec<NamedValue>;

/// Enum data
#[derive(Debug, Clone)]
pub struct Data {
    /// The enum type name
    name: Option<String>,

    /// The enum size, in bytes
    size: usize,

    /// A list of enum values
    named_value_list: NamedValueList,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &Header) -> usize {
        type_header.vlen() * ENUM_VALUE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        file_header: &FileHeader,
        type_header: &Header,
    ) -> BTFResult<Self> {
        let signed = type_header.kind_flag();
        let mut named_value_list = NamedValueList::new();

        for _ in 0..type_header.vlen() {
            let name_offset = reader.u32()?;
            let value_name = parse_string(reader, file_header, name_offset)?;

            let value = match signed {
                true => IntegerValue::Signed(reader.i32()?),
                false => IntegerValue::Unsigned(reader.u32()?),
            };

            named_value_list.push(NamedValue {
                name: value_name,
                value,
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
            named_value_list,
        })
    }
}

define_type!(Enum, Data,
    name: Option<String>,
    size: usize,
    named_value_list: NamedValueList
);

#[cfg(test)]
mod tests {
    use super::{Enum, IntegerValue};
    use crate::btf::{FileHeader, Header, Type};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_unsigned_enum() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x1C, 0x00, 0x00, 0x00, // type_len
            0x1C, 0x00, 0x00, 0x00, // str_off
            0x16, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x02, 0x00, 0x00, 0x06, // type header: info_flags
            0x04, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x07, 0x00, 0x00, 0x00, // First entry name offset
            0xFE, 0x00, 0x00, 0x00, // First entry value
            0x0E, 0x00, 0x00, 0x00, // Second entry name offset
            0xFE, 0x00, 0x00, 0x00, // Second entry value
            //
            // String section
            //
            0x00, // mandatory null string
            0x53, 0x74, 0x61, 0x74, 0x65, 0x00, // "State"
            0x50, 0x61, 0x75, 0x73, 0x65, 0x64, 0x00, // Paused
            0x52, 0x75, 0x6E, 0x6E, 0x69, 0x6E, 0x67, 0x00, // Running
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let r#enum = Enum::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(*r#enum.size(), 4);
        assert!(!r#enum.header().kind_flag());
        assert_eq!(r#enum.name().as_deref(), Some("State"));

        assert_eq!(r#enum.named_value_list().len(), 2);
        assert_eq!(r#enum.named_value_list()[0].name, "Paused");
        assert_eq!(
            r#enum.named_value_list()[0].value,
            IntegerValue::Unsigned(254)
        );

        assert_eq!(r#enum.named_value_list()[1].name, "Running");
        assert_eq!(
            r#enum.named_value_list()[1].value,
            IntegerValue::Unsigned(254)
        );
    }

    #[test]
    fn test_signed_enum() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x1C, 0x00, 0x00, 0x00, // type_len
            0x1C, 0x00, 0x00, 0x00, // str_off
            0x16, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x01, 0x00, 0x00, 0x00, // type header: name_offset
            0x02, 0x00, 0x00, 0x86, // type header: info_flags
            0x04, 0x00, 0x00, 0x00, // type header: size_or_type
            // Extra info
            0x07, 0x00, 0x00, 0x00, // First entry name offset
            0xFE, 0x00, 0x00, 0x00, // First entry value
            0x0E, 0x00, 0x00, 0x00, // Second entry name offset
            0xFE, 0x00, 0x00, 0x00, // Second entry value
            //
            // String section
            //
            0x00, // mandatory null string
            0x53, 0x74, 0x61, 0x74, 0x65, 0x00, // "State"
            0x50, 0x61, 0x75, 0x73, 0x65, 0x64, 0x00, // Paused
            0x52, 0x75, 0x6E, 0x6E, 0x69, 0x6E, 0x67, 0x00, // Running
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let r#enum = Enum::new(&mut reader, &file_header, type_header).unwrap();
        assert_eq!(*r#enum.size(), 4);
        assert!(r#enum.header().kind_flag());
        assert_eq!(r#enum.name().as_deref(), Some("State"));

        assert_eq!(r#enum.named_value_list().len(), 2);
        assert_eq!(r#enum.named_value_list()[0].name, "Paused");
        assert_eq!(
            r#enum.named_value_list()[0].value,
            IntegerValue::Signed(254)
        );

        assert_eq!(r#enum.named_value_list()[1].name, "Running");
        assert_eq!(
            r#enum.named_value_list()[1].value,
            IntegerValue::Signed(254)
        );
    }
}
