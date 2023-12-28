use crate::btf::{
    Array, Const, Enum, Enum64, Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Float,
    Func, FuncProto, Fwd, Int, Kind, Ptr, Readable, Restrict, Result as BTFResult, Struct, Type,
    TypeHeader, Typedef, Union, Var, Volatile,
};
use crate::generate_constructor_dispatcher;
use crate::utils::Reader;

use std::collections::BTreeMap;

/// An enum representing a BTF type
#[derive(Debug, Clone)]
pub enum TypeVariant {
    /// An integer type
    Int(Int),

    /// A typedef type
    Typedef(Typedef),

    /// A 32-bit enum type
    Enum(Enum),

    /// A pointer type
    Ptr(Ptr),

    /// A const type
    Const(Const),

    /// A volatile type
    Volatile(Volatile),

    /// An array type
    Array(Array),

    /// A function prototype
    FuncProto(FuncProto),

    /// A struct type
    Struct(Struct),

    /// A union type
    Union(Union),

    /// A forward declaration type
    Fwd(Fwd),

    /// A variable declaration
    Var(Var),

    /// A 64-bit enum type
    Enum64(Enum64),

    /// A function declaration
    Func(Func),

    /// A float type
    Float(Float),

    /// A restrict type
    Restrict(Restrict),
}

/// Returns the name of the given type
fn get_type_enum_value_name(type_var: &TypeVariant) -> Option<String> {
    match type_var {
        TypeVariant::Int(int) => int.name(),
        TypeVariant::Typedef(typedef) => typedef.name(),
        TypeVariant::Enum(r#enum) => r#enum.name(),
        TypeVariant::Struct(r#struct) => r#struct.name(),
        TypeVariant::Union(r#union) => r#union.name(),
        TypeVariant::Fwd(fwd) => fwd.name(),
        TypeVariant::Var(var) => var.name(),
        TypeVariant::Enum64(enum64) => enum64.name(),
        TypeVariant::Func(func) => func.name(),
        TypeVariant::Float(float) => float.name(),

        TypeVariant::Ptr(_)
        | TypeVariant::Const(_)
        | TypeVariant::Volatile(_)
        | TypeVariant::Array(_)
        | TypeVariant::FuncProto(_)
        | TypeVariant::Restrict(_) => None,
    }
}

/// Type information acquired from the BTF data
pub struct TypeInformation {
    /// Maps a type id to the type object
    id_to_type_map: BTreeMap<u32, TypeVariant>,

    /// Maps a type name to a type id
    name_to_id_map: BTreeMap<String, u32>,

    /// Maps a type id to a type name
    id_to_name_map: BTreeMap<u32, String>,
}

generate_constructor_dispatcher!(
    Int, Typedef, Enum, Ptr, Const, Volatile, Array, FuncProto, Struct, Union, Fwd, Var, Enum64,
    Func, Float, Restrict
);

impl TypeInformation {
    /// Creates a new `TypeInformation` object
    pub fn new(readable: &dyn Readable) -> BTFResult<Self> {
        let mut reader = Reader::new(readable);

        let file_header = FileHeader::new(&mut reader)?;
        let type_section_start = (file_header.hdr_len() + file_header.type_off()) as usize;
        let type_section_end = type_section_start + (file_header.type_len() as usize);

        reader.set_offset(type_section_start);

        let mut type_id_generator: u32 = 0;

        let mut id_to_type_map = BTreeMap::<u32, TypeVariant>::new();
        let mut name_to_id_map = BTreeMap::<String, u32>::new();
        let mut id_to_name_map = BTreeMap::<u32, String>::new();

        while reader.offset() < type_section_end {
            let type_header = TypeHeader::new(&mut reader, &file_header)?;
            let btf_type = parse_type(type_header.kind(), &mut reader, &file_header, type_header)?;

            let type_id = type_id_generator;
            type_id_generator += 1;

            if let Some(name) = get_type_enum_value_name(&btf_type) {
                name_to_id_map.insert(name.to_string(), type_id);
                id_to_name_map.insert(type_id, name.to_string());
            }

            id_to_type_map.insert(type_id, btf_type);
        }

        Ok(Self {
            id_to_type_map,
            name_to_id_map,
            id_to_name_map,
        })
    }

    /// Returns the entire type map
    pub fn type_map(&self) -> &BTreeMap<u32, TypeVariant> {
        &self.id_to_type_map
    }

    /// Returns the type id for the given type name
    pub fn type_id(&self, type_name: &str) -> Option<u32> {
        if type_name == "void" {
            return Some(0);
        }

        self.name_to_id_map.get(type_name).copied()
    }

    /// Returns the type object for the given type id
    pub fn type_object(&self, type_id: u32) -> Option<TypeVariant> {
        self.id_to_type_map.get(&type_id).cloned()
    }

    /// Returns the name of the given type id
    pub fn type_name(&self, type_id: u32) -> Option<String> {
        if type_id == 0 {
            return Some("void".to_string());
        }

        self.id_to_name_map.get(&type_id).cloned()
    }
}
