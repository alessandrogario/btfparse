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

generate_constructor_dispatcher!(
    Int, Typedef, Enum, Ptr, Const, Volatile, Array, FuncProto, Struct, Union, Fwd, Var, Enum64,
    Func, Float, Restrict, DataSec, TypeTag, DeclTag
);

impl TypeInformation {
    /// Creates a new `TypeInformation` object
    pub fn new(readable: &dyn Readable) -> BTFResult<Self> {
        let mut reader = Reader::new(readable);

        let file_header = FileHeader::new(&mut reader)?;
        let type_section_start = (file_header.hdr_len() + file_header.type_off()) as usize;
        let type_section_end = type_section_start + (file_header.type_len() as usize);

        reader.set_offset(type_section_start);

        let mut type_id_generator: u32 = 1;

        let mut id_to_type_map = BTreeMap::<u32, TypeVariant>::new();
        let mut name_to_id_map = BTreeMap::<String, u32>::new();
        let mut id_to_name_map = BTreeMap::<u32, String>::new();

        while reader.offset() < type_section_end {
            let type_header = Header::new(&mut reader, &file_header)?;
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
        if type_id == 0 {
            return Some(TypeVariant::Void);
        }

        self.id_to_type_map.get(&type_id).cloned()
    }

    /// Returns the name of the given type id
    pub fn type_name(&self, type_id: u32) -> Option<String> {
        if type_id == 0 {
            return Some("void".to_string());
        }

        self.id_to_name_map.get(&type_id).cloned()
    }

    /// Returns the size of the given type id
    pub fn type_size(&self, type_id: u32) -> BTFResult<usize> {
        let type_variant = self.type_object(type_id).ok_or(BTFError::new(
            BTFErrorKind::InvalidTypeID,
            "Invalid type id",
        ))?;

        match type_variant {
            TypeVariant::Void => Err(BTFError::new(
                BTFErrorKind::InvalidTypeID,
                "The void type has no size",
            )),

            TypeVariant::Ptr(_) => {
                let list_head_type_id = self.type_id("list_head").ok_or(BTFError::new(
                    BTFErrorKind::InvalidTypeID,
                    "The `struct list_head` type, used to extract the pointer size, was not found",
                ))?;

                let list_head_type_var = self.type_object(list_head_type_id).ok_or(
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
                let type_id = *array.element_type_id();
                let element_size = self.type_size(type_id)?;
                let element_count = *array.element_count() as usize;

                Ok(element_size * element_count)
            }

            TypeVariant::Float(float) => Ok(*float.size()),
            TypeVariant::Int(int) => Ok(*int.size()),
            TypeVariant::Enum(enm) => Ok(*enm.size()),
            TypeVariant::Enum64(enm) => Ok(*enm.size()),
            TypeVariant::Struct(str) => Ok(*str.size()),
            TypeVariant::Union(union) => Ok(*union.size()),

            TypeVariant::Typedef(typedef) => self.type_size(*typedef.type_id()),

            TypeVariant::Fwd(fwd) => self.type_size(*fwd.type_id()),

            TypeVariant::Const(cnst) => self.type_size(*cnst.type_id()),

            TypeVariant::Volatile(volatile) => self.type_size(*volatile.type_id()),

            TypeVariant::Restrict(restrict) => self.type_size(*restrict.type_id()),

            _ => Err(BTFError::new(
                BTFErrorKind::InvalidTypeID,
                &format!("Type {:?} has no size", type_variant),
            )),
        }
    }

    /// Returns the offset of the given type path
    pub fn offset_of_in_named_type(&self, type_name: &str, path: &str) -> BTFResult<usize> {
        let type_id = self.type_id(type_name).ok_or(BTFError::new(
            BTFErrorKind::InvalidTypeID,
            "The specified type id was not found",
        ))?;

        self.offset_of(type_id, path)
    }

    /// Returns the offset of the given type path
    pub fn offset_of(&self, type_id: u32, path: &str) -> BTFResult<usize> {
        let path_component_list = Self::split_path_components(path)?;
        self.offset_of_helper(0, type_id, path_component_list)
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
        mut type_id: u32,
        path: TypePath,
    ) -> BTFResult<usize> {
        if path.is_empty() {
            return Ok(offset);
        }

        let type_var = self.type_object(type_id).ok_or(BTFError::new(
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
                return self.offset_of_helper(offset, *fwd.type_id(), path);
            }

            TypeVariant::Typedef(typedef) => {
                return self.offset_of_helper(offset, *typedef.type_id(), path);
            }

            TypeVariant::Const(cnst) => {
                return self.offset_of_helper(offset, *cnst.type_id(), path);
            }

            TypeVariant::Volatile(volatile) => {
                return self.offset_of_helper(offset, *volatile.type_id(), path);
            }

            TypeVariant::Restrict(restrict) => {
                return self.offset_of_helper(offset, *restrict.type_id(), path);
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

                        let element_type_id = *array.element_type_id();
                        let element_type_size = self.type_size(element_type_id)?;
                        offset += index * element_type_size;

                        type_id = element_type_id;
                    }

                    TypeVariant::Ptr(ptr) => {
                        let pointee_type_id = *ptr.type_id();
                        let element_type_size = self.type_size(pointee_type_id)?;

                        offset += index * element_type_size;
                        type_id = pointee_type_id;
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
                        // Attempt to forward the request to any unnamed member (anonymous structs). If this
                        // succeeds, then we can just return the offset we get back, as it will consume the
                        // entire path.
                        if let Some(offset) = str.member_list().iter().find_map(|member| {
                            if member.name().is_none() {
                                match self.offset_of_helper(offset, member.type_id(), path.clone())
                                {
                                    Ok(offset) => Some(offset),
                                    Err(_) => None,
                                }
                            } else {
                                None
                            }
                        }) {
                            return Ok(offset);
                        }

                        // Try again, this time looking for a named member that matches the current
                        // path component. In this case, we need to consume the path component and
                        // continue the search.
                        let (next_type_id, member_offset) = str
                            .member_list()
                            .iter()
                            .find_map(|member| {
                                member.name().map(|member_name| {
                                    if *name == member_name {
                                        Some((member.type_id(), (member.offset() / 8) as usize))
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
                                        type_var, name
                                    ),
                                )
                            })?;

                        type_id = next_type_id;
                        offset += member_offset;
                    }

                    TypeVariant::Union(union) => {
                        // Attempt to forward the request to any unnamed member (anonymous structs). If this
                        // succeeds, then we can just return the offset we get back, as it will consume the
                        // entire path.
                        if let Some(offset) = union.member_list().iter().find_map(|member| {
                            if member.name().is_none() {
                                match self.offset_of_helper(offset, type_id, path.clone()) {
                                    Ok(offset) => Some(offset),
                                    Err(_) => None,
                                }
                            } else {
                                None
                            }
                        }) {
                            return Ok(offset);
                        }

                        // Try again, this time looking for a named member that matches the current
                        // path component. In this case, we need to consume the path component and
                        // continue the search.
                        let (next_type_id, member_offset) = union
                            .member_list()
                            .iter()
                            .find_map(|member| {
                                member.name().map(|member_name| {
                                    if *name == member_name {
                                        Some((member.type_id(), member.offset() as usize))
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
                                        type_var, name
                                    ),
                                )
                            })?;

                        type_id = next_type_id;
                        offset += member_offset;
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

        self.offset_of_helper(offset, type_id, path[1..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
