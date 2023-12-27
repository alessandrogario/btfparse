use crate::btf::parser::{parse_string, BTFHeader, TypeHeader, TypeKind};
use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};
use crate::utils::Reader;

/// The size of the extra data (one per enum value)
const ENUM_VALUE_SIZE: usize = 8;

/// Represents an enum value
#[derive(PartialEq, Eq, Debug)]
pub enum IntegerValue {
    /// The signed value
    Signed(i32),

    /// The unsigned value
    Unsigned(u32),
}

/// Represents a single enum value
pub struct NamedValue {
    /// The name of the value
    pub name: String,

    /// The signed value
    pub value: IntegerValue,
}

/// Represents a list of enum values
pub type NamedValueList = Vec<NamedValue>;

/// Represents an enum BTF type
pub struct Enum32 {
    /// The name of the type
    name: String,

    /// Whether the enum is signed or not
    signed: bool,

    /// The size of the type
    size: usize,

    /// The list of enum values
    named_value_list: NamedValueList,
}

impl Enum32 {
    /// Creates a new int type
    pub fn new(
        reader: &mut Reader,
        btf_header: &BTFHeader,
        type_header: &TypeHeader,
    ) -> BTFResult<Self> {
        let type_section_start = btf_header.hdr_len() + btf_header.type_off();
        let type_section_end = type_section_start + btf_header.type_len();

        let value_count = type_header.vlen();
        let required_size = value_count * ENUM_VALUE_SIZE;

        if reader.offset() + required_size > type_section_end as usize {
            return Err(BTFError::new(
                BTFErrorKind::InvalidTypeSectionOffset,
                "Invalid type section offset",
            ));
        }

        if type_header.kind() != TypeKind::Enum {
            return Err(BTFError::new(
                BTFErrorKind::InvalidBTFKind,
                "Not an enum type",
            ));
        }

        match type_header.size_or_type() {
            1 | 2 | 4 | 8 => {}
            _ => {
                return Err(BTFError::new(
                    BTFErrorKind::InvalidTypeHeaderAttribute,
                    "Invalid size_or_type attribute for enum type",
                ));
            }
        }

        let name = parse_string(reader, btf_header, type_header.name_offset())?;
        let signed = type_header.kind_flag();
        let size = type_header.size_or_type() as usize;

        let mut named_value_list = NamedValueList::new();

        for _ in 0..value_count {
            let name_offset = reader.u32()?;
            let value_name = parse_string(reader, btf_header, name_offset)?;

            let value = match signed {
                true => IntegerValue::Signed(reader.i32()?),
                false => IntegerValue::Unsigned(reader.u32()?),
            };

            named_value_list.push(NamedValue {
                name: value_name,
                value,
            });
        }

        Ok(Enum32 {
            name,
            signed,
            size,
            named_value_list,
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

    /// Returns whether the enum is signed or not
    pub fn signed(&self) -> bool {
        self.signed
    }

    /// Returns the enum value list
    pub fn enum_value_list(&self) -> &NamedValueList {
        &self.named_value_list
    }
}

#[cfg(test)]
mod tests {
    use super::Enum32;
    use crate::btf::parser::{BTFHeader, IntegerValue, TypeHeader};
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
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let enum32 = Enum32::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(enum32.size(), 4);
        assert!(!enum32.signed());
        assert_eq!(enum32.name(), "State");

        assert_eq!(enum32.enum_value_list().len(), 2);
        assert_eq!(enum32.enum_value_list()[0].name, "Paused");
        assert_eq!(
            enum32.enum_value_list()[0].value,
            IntegerValue::Unsigned(254)
        );

        assert_eq!(enum32.enum_value_list()[1].name, "Running");
        assert_eq!(
            enum32.enum_value_list()[1].value,
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
        let btf_header = BTFHeader::new(&mut reader).unwrap();
        let type_header = TypeHeader::new(&mut reader, &btf_header).unwrap();
        let enum32 = Enum32::new(&mut reader, &btf_header, &type_header).unwrap();
        assert_eq!(enum32.size(), 4);
        assert!(enum32.signed());
        assert_eq!(enum32.name(), "State");

        assert_eq!(enum32.enum_value_list().len(), 2);
        assert_eq!(enum32.enum_value_list()[0].name, "Paused");
        assert_eq!(enum32.enum_value_list()[0].value, IntegerValue::Signed(254));

        assert_eq!(enum32.enum_value_list()[1].name, "Running");
        assert_eq!(enum32.enum_value_list()[1].value, IntegerValue::Signed(254));
    }
}
