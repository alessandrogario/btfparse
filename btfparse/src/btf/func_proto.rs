use crate::btf::{
    parse_string, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Header, Kind,
    Result as BTFResult, Type,
};
use crate::utils::Reader;
use crate::{define_common_type_methods, define_type};

/// The size required to hold the extra data for a single parameter
const PARAMETER_VALUE_SIZE: usize = 8;

/// A single parameter for a function prototype
#[derive(Debug, Clone)]
pub struct Parameter {
    /// The raw string section offset
    name_offset: u32,

    /// The parameter name
    name: Option<String>,

    /// The parameter type id
    type_id: u32,
}

impl Parameter {
    /// Returns the raw string section offset
    pub fn name_offset(&self) -> u32 {
        self.name_offset
    }

    /// Returns the parameter name
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Returns the type id of the parameter
    pub fn type_id(&self) -> u32 {
        self.type_id
    }
}

/// A list of function prototype parameters
pub type ParameterList = Vec<Parameter>;

/// Func proto data
#[derive(Debug, Clone)]
pub struct Data {
    /// The full parameter list for this function prototype data
    parameter_list: ParameterList,

    /// Return type id
    return_type_id: u32,
}

impl Data {
    /// The size of the extra data
    pub fn size(type_header: &Header) -> usize {
        type_header.vlen() * PARAMETER_VALUE_SIZE
    }

    /// Creates a new `Data` object
    pub fn new(
        reader: &mut Reader,
        file_header: &FileHeader,
        type_header: &Header,
    ) -> BTFResult<Self> {
        let mut parameter_list = ParameterList::new();

        for _ in 0..type_header.vlen() {
            let name_offset = reader.u32()?;
            let type_id = reader.u32()?;

            let name = if name_offset != 0 {
                Some(parse_string(reader, file_header, name_offset)?)
            } else {
                None
            };

            parameter_list.push(Parameter {
                name_offset,
                name,
                type_id,
            });
        }

        Ok(Self {
            parameter_list,
            return_type_id: type_header.size_or_type(),
        })
    }
}

define_type!(FuncProto, Data, return_type_id: u32, parameter_list: ParameterList);

#[cfg(test)]
mod tests {
    use super::FuncProto;
    use crate::btf::{FileHeader, Header};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_func_proto() {
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
            0x0F, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x00, 0x00, 0x00, 0x00, // type header: name_offset
            0x02, 0x00, 0x00, 0x0D, // type header: info_flags
            0x05, 0x00, 0x00, 0x00, // type header: size_or_type
            //
            // Extra data
            //
            0x01, 0x00, 0x00, 0x00, // param 1: name offset
            0x02, 0x00, 0x00, 0x00, // param 1: type id
            0x08, 0x00, 0x00, 0x00, // param 2: name offset
            0x03, 0x00, 0x00, 0x00, // param 2: type id
            //
            // String section
            //
            0x00, // mandatory null string
            0x70, 0x61, 0x72, 0x61, 0x6D, 0x31, 0x00, // "param1"
            0x70, 0x61, 0x72, 0x61, 0x6D, 0x32, 0x00, // "param2"
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();
        let type_header = Header::new(&mut reader, &file_header).unwrap();
        let func_proto_type = FuncProto::new(&mut reader, &file_header, type_header).unwrap();

        assert_eq!(*func_proto_type.return_type_id(), 5);
        assert_eq!(func_proto_type.parameter_list().len(), 2);

        assert_eq!(func_proto_type.parameter_list()[0].type_id(), 2);
        assert_eq!(
            func_proto_type.parameter_list()[0].name().as_deref(),
            Some("param1")
        );

        assert_eq!(func_proto_type.parameter_list()[1].type_id(), 3);
        assert_eq!(
            func_proto_type.parameter_list()[1].name().as_deref(),
            Some("param2")
        );
    }
}
