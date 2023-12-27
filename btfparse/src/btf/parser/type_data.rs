use core::panic;
use std::collections::BTreeMap;

use crate::btf::parser::{
    Array, BTFHeader, Const, Enum32, Int, Ptr, Type, TypeHeader, TypeKind, Typedef, Volatile,
};
use crate::btf::{Readable, Result as BTFResult};
use crate::utils::Reader;

pub struct TypeData {
    pub name_to_id: BTreeMap<String, u32>,
    pub id_to_name: BTreeMap<u32, String>,
    pub id_to_type: BTreeMap<u32, Type>,
}

fn get_btf_type_name(btf_type: &Type) -> Option<String> {
    match btf_type {
        Type::Int(int) => Some(int.name().to_string()),
        Type::Enum(enum32) => Some(enum32.name().to_string()),
        Type::Typedef(typedef) => Some(typedef.name().to_string()),
        Type::Ptr(_) => None,
        Type::Array(_) => None,
        Type::Volatile(_) => None,
        Type::Const(_) => None,
    }
}

impl TypeData {
    /// Parses `readable` from start to finish
    pub fn new(readable: &dyn Readable) -> BTFResult<Self> {
        let mut reader = Reader::new(readable);
        let btf_header = BTFHeader::new(&mut reader)?;

        let start_offset = (btf_header.hdr_len() + btf_header.type_off()) as usize;
        let end_offset = start_offset + (btf_header.type_len() as usize);

        reader.set_offset(start_offset);

        let mut type_data = Self {
            name_to_id: BTreeMap::new(),
            id_to_name: BTreeMap::new(),
            id_to_type: BTreeMap::new(),
        };

        let mut type_id_generator: u32 = 1;

        loop {
            let remaining_bytes = end_offset - reader.offset();
            if remaining_bytes == 0 {
                break;
            }

            let type_header = TypeHeader::new(&mut reader, &btf_header)?;

            let btf_type = match type_header.kind() {
                TypeKind::Int => Type::Int(Int::new(&mut reader, &btf_header, &type_header)?),
                TypeKind::Ptr => Type::Ptr(Ptr::new(&type_header)?),
                TypeKind::Array => Type::Array(Array::new(&mut reader, &btf_header, &type_header)?),
                TypeKind::Enum => Type::Enum(Enum32::new(&mut reader, &btf_header, &type_header)?),
                TypeKind::Typedef => {
                    Type::Typedef(Typedef::new(&mut reader, &btf_header, &type_header)?)
                }
                TypeKind::Const => Type::Const(Const::new(&type_header)?),
                TypeKind::Volatile => Type::Volatile(Volatile::new(&type_header)?),

                _ => {
                    panic!("Unsupported type: {:?}", type_header.kind());
                }
            };

            let type_id = type_id_generator;
            type_id_generator += 1;

            if let Some(type_name) = get_btf_type_name(&btf_type) {
                type_data.id_to_name.insert(type_id, type_name.clone());
                type_data.name_to_id.insert(type_name, type_id);
            }

            type_data.id_to_type.insert(type_id, btf_type);
        }

        Ok(type_data)
    }
}
