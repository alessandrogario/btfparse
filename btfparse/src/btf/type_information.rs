use crate::btf::{
    Array, Const, DataSec, DeclTag, Enum, Enum64, Error as BTFError, ErrorKind as BTFErrorKind,
    FileHeader, Float, Func, FuncProto, Fwd, Header, Int, Kind, Ptr, Readable, Restrict,
    Result as BTFResult, Struct, TypeTag, Typedef, Union, Var, Volatile,
};
use crate::generate_constructor_dispatcher;
use crate::utils::Reader;

use std::collections::BTreeMap;

/// An enum representing a BTF type
#[derive(Debug, Clone)]
pub enum TypeVariant {
    /// The void type
    Void,

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

    /// A data section decl
    DataSec(DataSec),

    /// A type tag
    TypeTag(TypeTag),

    /// A decl tag
    DeclTag(DeclTag),
}

/// Returns the name of the given type
fn get_type_enum_value_name(type_var: &TypeVariant) -> Option<String> {
    match type_var {
        TypeVariant::Void => Some("void".to_string()),
        TypeVariant::Int(int) => int.name().clone(),
        TypeVariant::Typedef(typedef) => typedef.name().clone(),
        TypeVariant::Enum(r#enum) => r#enum.name().clone(),
        TypeVariant::Struct(r#struct) => r#struct.name().clone(),
        TypeVariant::Union(r#union) => r#union.name().clone(),
        TypeVariant::Fwd(fwd) => fwd.name().clone(),
        TypeVariant::Var(var) => var.name().clone(),
        TypeVariant::Enum64(enum64) => enum64.name().clone(),
        TypeVariant::Func(func) => func.name().clone(),
        TypeVariant::Float(float) => float.name().clone(),
        TypeVariant::DataSec(data_sec) => data_sec.name().clone(),
        TypeVariant::TypeTag(type_tag) => type_tag.name().clone(),
        TypeVariant::DeclTag(decl_tag) => decl_tag.name().clone(),

        TypeVariant::Ptr(_)
        | TypeVariant::Const(_)
        | TypeVariant::Volatile(_)
        | TypeVariant::Array(_)
        | TypeVariant::FuncProto(_)
        | TypeVariant::Restrict(_) => None,
    }
}

/// A component of a type path
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypePathComponent {
    /// An index into an array
    Index(usize),

    /// A name of a struct (or union) field
    Name(String),
}

/// A list of path components
pub type TypePath = Vec<TypePathComponent>;

/// Tracks the internal state of the type path parser
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TypePathParserState {
    /// The initial (empty) state
    Start,

    /// Inside the first character of a field name
    InsideFirstCharacterOfName,

    /// Inside a field name
    InsideName,

    /// Inside an index
    InsideIndex,

    /// After an index
    AfterIndex,
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

// Generate the parse_type functions for each type
generate_constructor_dispatcher!(
    Int, Typedef, Enum, Ptr, Const, Volatile, Array, FuncProto, Struct, Union, Fwd, Var, Enum64,
    Func, Float, Restrict, DataSec, TypeTag, DeclTag
);

/// This macro is used to generate the `offset_of` method for structs and unions
macro_rules! offset_of_struct_and_union_helper {
    ($self:ident, $current_offset:ident, $current_type:ident, $struct_or_union:ident, $name:ident, $path:ident) => {
        // Attempt to forward the request to any unnamed member (anonymous structs). If this
        // succeeds then we can just return the offset we get back, as it will consume the
        // entire path.
        if let Some(final_offset) = $struct_or_union.member_list().iter().find_map(|member| {
            if member.name().is_none() {
                match $self.offset_of_helper($current_offset, member.tid(), $path.clone()) {
                    Ok(inner_member_offset) => Some(member.offset() as usize + inner_member_offset),
                    Err(_) => None,
                }
            } else {
                None
            }
        }) {
            return Ok($current_offset + final_offset);
        }

        // Try again, this time looking for a named member that matches the current
        // path component
        let (next_tid, member_offset) = $struct_or_union
            .member_list()
            .iter()
            .find_map(|member| {
                member.name().map(|member_name| {
                    if *$name == member_name {
                        Some((member.tid(), member.offset() as usize))
                    } else {
                        None
                    }
                })?
            })
            .ok_or_else(|| {
                BTFError::new(
                    BTFErrorKind::InvalidTypePath,
                    &format!(
                        "Type {:?} does not have a member named {}",
                        $struct_or_union, $name
                    ),
                )
            })?;

        $current_type = next_tid;
        $current_offset += member_offset;
    };
}

impl TypeInformation {
    /// Creates a new `TypeInformation` object
    pub fn new(readable: &dyn Readable) -> BTFResult<Self> {
        let mut reader = Reader::new(readable);

        let file_header = FileHeader::new(&mut reader)?;
        let type_section_start = (file_header.hdr_len() + file_header.type_off()) as usize;
        let type_section_end = type_section_start + (file_header.type_len() as usize);

        reader.set_offset(type_section_start);

        let mut tid_generator: u32 = 1;

        let mut id_to_type_map = BTreeMap::<u32, TypeVariant>::new();
        let mut name_to_id_map = BTreeMap::<String, u32>::new();
        let mut id_to_name_map = BTreeMap::<u32, String>::new();

        while reader.offset() < type_section_end {
            let type_header = Header::new(&mut reader, &file_header)?;
            let btf_type = parse_type(type_header.kind(), &mut reader, &file_header, type_header)?;

            let tid = tid_generator;
            tid_generator += 1;

            if let Some(name) = get_type_enum_value_name(&btf_type) {
                name_to_id_map.insert(name.to_string(), tid);
                id_to_name_map.insert(tid, name.to_string());
            }

            id_to_type_map.insert(tid, btf_type);
        }

        Ok(Self {
            id_to_type_map,
            name_to_id_map,
            id_to_name_map,
        })
    }

    /// Returns the entire type map
    pub fn get(&self) -> &BTreeMap<u32, TypeVariant> {
        &self.id_to_type_map
    }

    /// Returns the type id for the given type name
    pub fn id_of(&self, type_name: &str) -> Option<u32> {
        if type_name == "void" {
            return Some(0);
        }

        self.name_to_id_map.get(type_name).copied()
    }

    /// Returns the type object for the given type id
    pub fn from_id(&self, tid: u32) -> Option<TypeVariant> {
        if tid == 0 {
            return Some(TypeVariant::Void);
        }

        self.id_to_type_map.get(&tid).cloned()
    }

    /// Returns the name of the given type id
    pub fn name_of(&self, tid: u32) -> Option<String> {
        if tid == 0 {
            return Some("void".to_string());
        }

        self.id_to_name_map.get(&tid).cloned()
    }

    /// Returns the size of the given type id
    pub fn size_of(&self, tid: u32) -> BTFResult<usize> {
        let type_variant = self.from_id(tid).ok_or(BTFError::new(
            BTFErrorKind::InvalidTypeID,
            "Invalid type id",
        ))?;

        match type_variant {
            TypeVariant::Ptr(_) => {
                let list_head_tid = self.id_of("list_head").ok_or(BTFError::new(
                    BTFErrorKind::InvalidTypeID,
                    "The `struct list_head` type, used to extract the pointer size, was not found",
                ))?;

                let list_head_type_var = self.from_id(list_head_tid).ok_or(
                    BTFError::new(BTFErrorKind::InvalidTypeID, "The extracted `struct list_head` type ID, used to extract the pointer size, was invalid"),
                )?;

                let list_head_type_size = match list_head_type_var {
                    TypeVariant::Struct(str) => Ok(*str.size()),

                    _ => {
                        Err(BTFError::new(BTFErrorKind::InvalidTypeID, "The extracted `struct list_head` type ID, used to extract the pointer size, is not a struct type"))
                    }
                }?;

                Ok(list_head_type_size / 2)
            }

            TypeVariant::Array(array) => {
                let tid = *array.element_tid();
                let element_size = self.size_of(tid)?;
                let element_count = *array.element_count() as usize;

                Ok(element_size * element_count)
            }

            TypeVariant::Float(float) => Ok(*float.size()),
            TypeVariant::Int(int) => Ok(*int.size()),
            TypeVariant::Enum(enm) => Ok(*enm.size()),
            TypeVariant::Enum64(enm) => Ok(*enm.size()),
            TypeVariant::Struct(str) => Ok(*str.size()),
            TypeVariant::Union(union) => Ok(*union.size()),
            TypeVariant::DataSec(data_sec) => Ok(*data_sec.size()),

            TypeVariant::Var(var) => self.size_of(*var.tid()),
            TypeVariant::Typedef(typedef) => self.size_of(*typedef.tid()),
            TypeVariant::Const(cnst) => self.size_of(*cnst.tid()),
            TypeVariant::Volatile(volatile) => self.size_of(*volatile.tid()),
            TypeVariant::Restrict(restrict) => self.size_of(*restrict.tid()),
            TypeVariant::TypeTag(type_tag) => self.size_of(*type_tag.tid()),

            _ => Err(BTFError::new(
                BTFErrorKind::NotSized,
                &format!("Type {:?} has no size", type_variant),
            )),
        }
    }

    /// Returns the offset of the given type path
    pub fn offset_of_in_named_type(&self, type_name: &str, path: &str) -> BTFResult<usize> {
        let tid = self.id_of(type_name).ok_or(BTFError::new(
            BTFErrorKind::InvalidTypeID,
            "The specified type id was not found",
        ))?;

        self.offset_of(tid, path)
    }

    /// Returns the offset of the given type path
    pub fn offset_of(&self, tid: u32, path: &str) -> BTFResult<usize> {
        let path_component_list = Self::split_path_components(path)?;
        self.offset_of_helper(0, tid, path_component_list)
    }

    /// Splits the given type path into its components
    fn split_path_components(path: &str) -> BTFResult<TypePath> {
        let mut path_component_list = TypePath::new();

        let mut buffer = String::new();
        let mut state = TypePathParserState::Start;

        let save_buffer = |state: &mut TypePathParserState,
                           buffer: &mut String,
                           path_component_list: &mut TypePath|
         -> BTFResult<()> {
            match state {
                TypePathParserState::InsideIndex => {
                    if buffer.is_empty() {
                        return Err(BTFError::new(BTFErrorKind::InvalidTypePath, "Empty index"));
                    }
                    let index = buffer.parse::<usize>().unwrap();
                    path_component_list.push(TypePathComponent::Index(index));
                }

                TypePathParserState::InsideName => {
                    path_component_list.push(TypePathComponent::Name(buffer.clone()));
                }

                _ => {
                    return Err(BTFError::new(
                        BTFErrorKind::InvalidTypePath,
                        "Invalid state",
                    ));
                }
            }

            *buffer = String::new();
            Ok(())
        };

        for (index, c) in path.chars().enumerate() {
            match state {
                TypePathParserState::Start => {
                    if c == '[' {
                        state = TypePathParserState::InsideIndex;
                    } else if c.is_alphabetic() {
                        buffer.push(c);
                        state = TypePathParserState::InsideName;
                    } else {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypePath,
                            &format!("Invalid character at index {}", index),
                        ));
                    }

                    continue;
                }

                TypePathParserState::InsideFirstCharacterOfName => {
                    if !c.is_alphabetic() {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypePath,
                            &format!("Invalid character at index {}", index),
                        ));
                    }

                    buffer.push(c);
                    state = TypePathParserState::InsideName;
                }

                TypePathParserState::InsideName => {
                    if c == '[' {
                        save_buffer(&mut state, &mut buffer, &mut path_component_list)?;
                        state = TypePathParserState::InsideIndex;
                    } else if c == '.' {
                        save_buffer(&mut state, &mut buffer, &mut path_component_list)?;
                        state = TypePathParserState::InsideFirstCharacterOfName;
                    } else if c.is_alphanumeric() || c == '_' {
                        buffer.push(c);
                    } else {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypePath,
                            &format!("Invalid character at index {}", index),
                        ));
                    }
                }

                TypePathParserState::InsideIndex => {
                    if c == ']' {
                        save_buffer(&mut state, &mut buffer, &mut path_component_list)?;
                        state = TypePathParserState::AfterIndex;
                    } else if c.is_numeric() {
                        buffer.push(c);
                    } else {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypePath,
                            &format!("Invalid character at index {}", index),
                        ));
                    }
                }

                TypePathParserState::AfterIndex => {
                    if c == '[' {
                        state = TypePathParserState::InsideIndex;
                    } else if c == '.' {
                        state = TypePathParserState::InsideFirstCharacterOfName;
                    } else {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypePath,
                            &format!("Invalid character at index {}", index),
                        ));
                    }
                }
            }
        }

        if !buffer.is_empty() {
            save_buffer(&mut state, &mut buffer, &mut path_component_list)?;
        }

        Ok(path_component_list)
    }

    /// Internal helper method for `TypeInformation::offset_of`
    fn offset_of_helper(
        &self,
        mut offset: usize,
        mut tid: u32,
        path: TypePath,
    ) -> BTFResult<usize> {
        if path.is_empty() {
            return Ok(offset);
        }

        let type_var = self.from_id(tid).ok_or(BTFError::new(
            BTFErrorKind::InvalidTypeID,
            "Invalid type id",
        ))?;

        match type_var {
            TypeVariant::Void => {
                return Err(BTFError::new(
                    BTFErrorKind::UnsupportedType,
                    "The void type can't be dereferenced with a path",
                ));
            }

            TypeVariant::Fwd(fwd) => {
                return self.offset_of_helper(offset, *fwd.tid(), path);
            }

            TypeVariant::Typedef(typedef) => {
                return self.offset_of_helper(offset, *typedef.tid(), path);
            }

            TypeVariant::Const(cnst) => {
                return self.offset_of_helper(offset, *cnst.tid(), path);
            }

            TypeVariant::Volatile(volatile) => {
                return self.offset_of_helper(offset, *volatile.tid(), path);
            }

            TypeVariant::Restrict(restrict) => {
                return self.offset_of_helper(offset, *restrict.tid(), path);
            }

            _ => {}
        }

        match &path[0] {
            TypePathComponent::Index(index) => {
                let index = *index;

                match &type_var {
                    TypeVariant::Array(array) => {
                        let element_count = *array.element_count() as usize;
                        if index >= element_count {
                            return Err(BTFError::new(
                                BTFErrorKind::InvalidTypePath,
                                &format!(
                                    "Index {} is out of bounds for array of size {}",
                                    index,
                                    array.element_count()
                                ),
                            ));
                        }

                        let element_tid = *array.element_tid();
                        let element_type_size = self.size_of(element_tid)?;
                        offset += index * element_type_size;

                        tid = element_tid;
                    }

                    TypeVariant::Ptr(ptr) => {
                        let pointee_tid = *ptr.tid();
                        let element_type_size = self.size_of(pointee_tid)?;

                        offset += index * element_type_size;
                        tid = pointee_tid;
                    }

                    _ => {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypePath,
                            &format!("Type {:?} is not indexable", type_var),
                        ));
                    }
                };
            }

            TypePathComponent::Name(name) => {
                match &type_var {
                    TypeVariant::Struct(str) => {
                        offset_of_struct_and_union_helper!(self, offset, tid, str, name, path);
                    }

                    TypeVariant::Union(union) => {
                        offset_of_struct_and_union_helper!(self, offset, tid, union, name, path);
                    }

                    _ => {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypePath,
                            &format!("Type {:?} is not a struct or union", type_var),
                        ));
                    }
                };
            }
        };

        self.offset_of_helper(offset, tid, path[1..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::btf::{
        data_sec::Variable as DataSecVariable,
        enum64::{IntegerValue as IntegerValue64, NamedValue as NamedValue64},
        r#enum::{IntegerValue as IntegerValue32, NamedValue as NamedValue32},
        struct_union::Member as StructMember,
        LinkageType,
    };

    #[test]
    fn test_split_path_components() {
        let type_path = TypeInformation::split_path_components("").unwrap();
        assert!(type_path.is_empty());

        let type_path = TypeInformation::split_path_components("[1]").unwrap();
        assert_eq!(type_path.len(), 1);
        assert_eq!(type_path[0], TypePathComponent::Index(1));

        let type_path = TypeInformation::split_path_components("[1][2]").unwrap();
        assert_eq!(type_path.len(), 2);
        assert_eq!(type_path[0], TypePathComponent::Index(1));
        assert_eq!(type_path[1], TypePathComponent::Index(2));

        let type_path = TypeInformation::split_path_components("test").unwrap();
        assert_eq!(type_path.len(), 1);
        assert_eq!(type_path[0], TypePathComponent::Name("test".to_string()));

        let type_path = TypeInformation::split_path_components("array[10]").unwrap();
        assert_eq!(type_path.len(), 2);
        assert_eq!(type_path[0], TypePathComponent::Name("array".to_string()));
        assert_eq!(type_path[1], TypePathComponent::Index(10));

        let type_path = TypeInformation::split_path_components("array[10].array2[11]").unwrap();
        assert_eq!(type_path.len(), 4);
        assert_eq!(type_path[0], TypePathComponent::Name("array".to_string()));
        assert_eq!(type_path[1], TypePathComponent::Index(10));
        assert_eq!(type_path[2], TypePathComponent::Name("array2".to_string()));
        assert_eq!(type_path[3], TypePathComponent::Index(11));

        TypeInformation::split_path_components(".value").unwrap_err();
        TypeInformation::split_path_components(".[10]").unwrap_err();
        TypeInformation::split_path_components("[value").unwrap_err();
        TypeInformation::split_path_components("]value").unwrap_err();
        TypeInformation::split_path_components("1").unwrap_err();
        TypeInformation::split_path_components("array[10]value").unwrap_err();
        TypeInformation::split_path_components("array[]").unwrap_err();
        TypeInformation::split_path_components("[]").unwrap_err();
    }

    fn get_test_type_info() -> TypeInformation {
        let mut type_info = TypeInformation {
            id_to_type_map: BTreeMap::<u32, TypeVariant>::new(),
            name_to_id_map: BTreeMap::<String, u32>::new(),
            id_to_name_map: BTreeMap::<u32, String>::new(),
        };

        // tid:1 BTF_KIND_INT
        type_info.id_to_type_map.insert(
            1,
            TypeVariant::Int(Int::create(
                Header::create(Kind::Int, 1, 0, false, 4),
                Some(String::from("unsigned int")),
                4,
                false,
                false,
                false,
                0,
                4,
            )),
        );

        type_info
            .name_to_id_map
            .insert(String::from("unsigned int"), 1);

        type_info
            .id_to_name_map
            .insert(1, String::from("unsigned int"));

        // tid:2 BTF_KIND_PTR
        type_info.id_to_type_map.insert(
            2,
            TypeVariant::Ptr(Ptr::create(Header::create(Kind::Ptr, 0, 0, false, 0), 0)),
        );

        // tid:3 BTF_KIND_ARRAY
        type_info.id_to_type_map.insert(
            3,
            TypeVariant::Array(Array::create(
                Header::create(Kind::Array, 0, 0, false, 0),
                1,
                1,
                10,
            )),
        );

        //
        // BTF_KIND_STRUCT
        //

        // tid:4 Anonymous struct type
        type_info.id_to_type_map.insert(
            4,
            TypeVariant::Struct(Struct::create(
                Header::create(Kind::Struct, 1, 2, false, 8),
                None,
                8,
                vec![
                    StructMember::create(1, Some(String::from("anon_struct_value1")), 1, 0),
                    StructMember::create(1, Some(String::from("anon_struct_value2")), 1, 32),
                ],
            )),
        );

        // tid:5 Anonymous union type
        type_info.id_to_type_map.insert(
            5,
            TypeVariant::Union(Union::create(
                Header::create(Kind::Union, 1, 2, true, 8),
                None,
                8,
                vec![
                    StructMember::create(1, Some(String::from("anon_union_value1")), 1, 0),
                    StructMember::create(1, Some(String::from("anon_union_value2")), 2, 0),
                ],
            )),
        );

        // tid:6 Named struct type
        type_info.id_to_type_map.insert(
            6,
            TypeVariant::Struct(Struct::create(
                Header::create(Kind::Struct, 1, 4, false, 28),
                Some(String::from("Struct")),
                28,
                vec![
                    StructMember::create(0, None, 4, 0),
                    StructMember::create(0, None, 5, 64),
                    StructMember::create(1, Some(String::from("int_value")), 1, 128),
                    StructMember::create(1, Some(String::from("ptr_value")), 2, 160),
                ],
            )),
        );

        type_info.name_to_id_map.insert(String::from("Struct"), 6);
        type_info.id_to_name_map.insert(6, String::from("Struct"));

        // tid:7 list_head struct, used internally to determine the size of a pointer
        type_info.id_to_type_map.insert(
            7,
            TypeVariant::Struct(Struct::create(
                Header::create(Kind::Struct, 1, 2, false, 16),
                Some(String::from("list_head")),
                16,
                vec![
                    StructMember::create(1, Some(String::from("next")), 2, 0),
                    StructMember::create(1, Some(String::from("prev")), 2, 64),
                ],
            )),
        );

        type_info
            .name_to_id_map
            .insert(String::from("list_head"), 7);

        type_info
            .id_to_name_map
            .insert(7, String::from("list_head"));

        // tid:8 BTF_KIND_ENUM
        type_info.id_to_type_map.insert(
            8,
            TypeVariant::Enum(Enum::create(
                Header::create(Kind::Enum, 1, 2, false, 4),
                Some(String::from("Enum32")),
                4,
                vec![NamedValue32 {
                    name: String::from("Enum32Value1"),
                    value: IntegerValue32::Unsigned(0),
                }],
            )),
        );

        type_info.name_to_id_map.insert(String::from("Enum32"), 8);
        type_info.id_to_name_map.insert(8, String::from("Enum32"));

        // tid:9 BTF_KIND_ENUM64
        type_info.id_to_type_map.insert(
            9,
            TypeVariant::Enum64(Enum64::create(
                Header::create(Kind::Enum64, 1, 2, false, 8),
                Some(String::from("Enum64")),
                8,
                vec![NamedValue64 {
                    name: String::from("Enum64Value1"),
                    value: IntegerValue64::Unsigned(0),
                }],
            )),
        );

        type_info.name_to_id_map.insert(String::from("Enum64"), 9);
        type_info.id_to_name_map.insert(9, String::from("Enum64"));

        // tid:10 BTF_KIND_FWD
        type_info.id_to_type_map.insert(
            10,
            TypeVariant::Fwd(Fwd::create(
                Header::create(Kind::Fwd, 1, 0, false, 6),
                Some(String::from("Fwd")),
                6,
            )),
        );

        type_info
            .name_to_id_map
            .insert(String::from("StructForwardDecl"), 10);
        type_info
            .id_to_name_map
            .insert(10, String::from("StructForwardDecl"));

        // tid:11 BTF_KIND_TYPEDEF
        type_info.id_to_type_map.insert(
            11,
            TypeVariant::Typedef(Typedef::create(
                Header::create(Kind::Typedef, 1, 2, false, 6),
                6,
                Some(String::from("StructAlias")),
            )),
        );

        type_info
            .name_to_id_map
            .insert(String::from("StructAlias"), 11);

        type_info
            .id_to_name_map
            .insert(11, String::from("StructAlias"));

        // tid:12 BTF_KIND_VOLATILE
        type_info.id_to_type_map.insert(
            12,
            TypeVariant::Volatile(Volatile::create(
                Header::create(Kind::Volatile, 0, 0, false, 1),
                1,
            )),
        );

        // tid:13 BTF_KIND_CONST
        type_info.id_to_type_map.insert(
            13,
            TypeVariant::Const(Const::create(
                Header::create(Kind::Const, 0, 0, false, 1),
                1,
            )),
        );

        // tid:14 BTF_KIND_RESTRICT
        type_info.id_to_type_map.insert(
            14,
            TypeVariant::Restrict(Restrict::create(
                Header::create(Kind::Restrict, 0, 0, false, 1),
                1,
            )),
        );

        // tid:15 BTF_KIND_FUNC
        type_info.id_to_type_map.insert(
            15,
            TypeVariant::Func(Func::create(
                Header::create(Kind::Func, 1, 0, false, 16),
                Some(String::from("func")),
                16,
            )),
        );

        type_info.name_to_id_map.insert(String::from("func"), 15);
        type_info.id_to_name_map.insert(15, String::from("func"));

        // tid:16 BTF_KIND_FUNC_PROTO
        type_info.id_to_type_map.insert(
            16,
            TypeVariant::FuncProto(FuncProto::create(
                Header::create(Kind::FuncProto, 0, 0, false, 1),
                1,
                vec![],
            )),
        );

        // tid:17 BTF_KIND_VAR
        type_info.id_to_type_map.insert(
            17,
            TypeVariant::Var(Var::create(
                Header::create(Kind::Var, 1, 0, false, 1),
                Some(String::from("var")),
                1,
                0,
                LinkageType::Global,
            )),
        );

        type_info.name_to_id_map.insert(String::from("var"), 17);
        type_info.id_to_name_map.insert(17, String::from("var"));

        // tid:18 BTF_KIND_DATASEC
        type_info.id_to_type_map.insert(
            18,
            TypeVariant::DataSec(DataSec::create(
                Header::create(Kind::DataSec, 1, 3, false, 12),
                Some(String::from(".data")),
                12,
                vec![
                    DataSecVariable {
                        var_decl_id: 17,
                        offset: 0,
                        var_size: 4,
                    },
                    DataSecVariable {
                        var_decl_id: 17,
                        offset: 4,
                        var_size: 4,
                    },
                    DataSecVariable {
                        var_decl_id: 17,
                        offset: 8,
                        var_size: 4,
                    },
                ],
            )),
        );

        type_info.name_to_id_map.insert(String::from(".data"), 18);
        type_info.id_to_name_map.insert(18, String::from(".data"));

        // tid:19 BTF_KIND_FLOAT
        type_info.id_to_type_map.insert(
            19,
            TypeVariant::Float(Float::create(
                Header::create(Kind::Float, 1, 0, false, 8),
                Some(String::from("double")),
                8,
            )),
        );

        type_info.name_to_id_map.insert(String::from("double"), 19);
        type_info.id_to_name_map.insert(19, String::from("double"));

        // tid:20 BTF_KIND_DECL_TAG
        type_info.id_to_type_map.insert(
            20,
            TypeVariant::DeclTag(DeclTag::create(
                Header::create(Kind::DeclTag, 1, 0, false, 6),
                Some(String::from("decl_tag")),
                6,
                0,
            )),
        );

        type_info
            .name_to_id_map
            .insert(String::from("decl_tag"), 20);

        type_info
            .id_to_name_map
            .insert(20, String::from("decl_tag"));

        // tid:21 BTF_KIND_TYPE_TAG
        type_info.id_to_type_map.insert(
            21,
            TypeVariant::TypeTag(TypeTag::create(
                Header::create(Kind::TypeTag, 1, 0, false, 11),
                Some(String::from("type_tag")),
                11,
            )),
        );

        type_info
            .name_to_id_map
            .insert(String::from("type_tag"), 21);

        type_info
            .id_to_name_map
            .insert(21, String::from("type_tag"));

        type_info
    }

    #[test]
    fn test_size_of() {
        let type_info = get_test_type_info();

        // The void type has no size
        assert_eq!(type_info.name_of(0).unwrap(), "void");
        assert!(type_info.size_of(0).unwrap_err().kind() == BTFErrorKind::NotSized);

        // The int type has a size of 4
        assert_eq!(type_info.size_of(1).unwrap(), 4);

        // The ptr size is half the size of the `list_head` struct
        assert_eq!(type_info.size_of(2).unwrap(), 8);

        // The array has 10 u32 values
        assert_eq!(type_info.size_of(3).unwrap(), 40);

        // The anonymous struct is 8 bytes
        assert_eq!(type_info.size_of(4).unwrap(), 8);

        // The anonymous union is 8 bytes
        assert_eq!(type_info.size_of(5).unwrap(), 8);

        // The named struct is 28 bytes
        assert_eq!(type_info.size_of(6).unwrap(), 28);

        // The internal `list_head` struct is 16 bytes. This is used to determine the size of a ptr
        assert_eq!(type_info.size_of(7).unwrap(), 16);

        // The enum is 4 bytes
        assert_eq!(type_info.size_of(8).unwrap(), 4);

        // The enum64 is 8 bytes
        assert_eq!(type_info.size_of(9).unwrap(), 8);

        // A forward declaration of an undefined type can't be sized
        assert_eq!(type_info.name_of(10).unwrap(), "StructForwardDecl");
        assert!(type_info.size_of(10).unwrap_err().kind() == BTFErrorKind::NotSized);

        // The typedef points to the named struct, which is 28 bytes
        assert_eq!(type_info.size_of(11).unwrap(), 28);

        // The volatile points to the int, which is 4 bytes
        assert_eq!(type_info.size_of(12).unwrap(), 4);

        // The const points to the int, which is 4 bytes
        assert_eq!(type_info.size_of(13).unwrap(), 4);

        // The restrict points to the int, which is 4 bytes
        assert_eq!(type_info.size_of(14).unwrap(), 4);

        // The BTF_KIND_FUNC has no size because it is not a type
        assert_eq!(type_info.name_of(15).unwrap(), "func");
        assert!(type_info.size_of(15).unwrap_err().kind() == BTFErrorKind::NotSized);

        // The BTF_KIND_FUNC_PROTO has no size
        assert!(type_info.size_of(16).unwrap_err().kind() == BTFErrorKind::NotSized);

        // The BTF_KIND_VAR variable has the size of its type (32-bit unsigned int)
        assert_eq!(type_info.size_of(17).unwrap(), 4);

        // The BTF_KIND_DATASEC variable has the size of its types (3x 32-bit unsigned int)
        assert_eq!(type_info.size_of(18).unwrap(), 12);

        // The BTF_KIND_FLOAT has a size of 8 bytes
        assert_eq!(type_info.size_of(19).unwrap(), 8);

        // The BTF_KIND_DECL_TAG has no size
        assert!(type_info.size_of(20).unwrap_err().kind() == BTFErrorKind::NotSized);

        // The BTF_KIND_DECL_TAG has size of the type it is applied to (named struct)
        assert_eq!(type_info.size_of(21).unwrap(), 28);
    }

    #[test]
    fn test_offset_of() {
        let type_info = get_test_type_info();
        let int_value_offset = type_info
            .offset_of(type_info.id_of("Struct").unwrap(), "int_value")
            .unwrap();

        assert_eq!(int_value_offset, 16 * 8);

        let ptr_value_offset = type_info
            .offset_of(type_info.id_of("Struct").unwrap(), "ptr_value")
            .unwrap();

        assert_eq!(ptr_value_offset, 20 * 8);

        let anon_struct_value1_offset = type_info
            .offset_of(type_info.id_of("Struct").unwrap(), "anon_struct_value1")
            .unwrap();

        assert_eq!(anon_struct_value1_offset, 0);

        let anon_struct_value2_offset = type_info
            .offset_of(type_info.id_of("Struct").unwrap(), "anon_struct_value2")
            .unwrap();

        assert_eq!(anon_struct_value2_offset, 4 * 8);

        let anon_union_value1_offset = type_info
            .offset_of(type_info.id_of("Struct").unwrap(), "anon_union_value1")
            .unwrap();

        assert_eq!(anon_union_value1_offset, 8 * 8);

        let anon_union_value2_offset = type_info
            .offset_of(type_info.id_of("Struct").unwrap(), "anon_union_value2")
            .unwrap();

        assert_eq!(anon_union_value2_offset, 8 * 8);
    }
}
