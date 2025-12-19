/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::{
    btf::{
        Array, Const, DataSec, DeclTag, Enum, Enum64, Error as BTFError, ErrorKind as BTFErrorKind,
        FileHeader, Float, Func, FuncProto, Fwd, Header, Int, Kind, Offset, Ptr, Readable,
        Restrict, Result as BTFResult, Struct, TypeTag, Typedef, Union, Var, Volatile,
    },
    generate_constructor_dispatcher,
    utils::Reader,
};

use std::{collections::BTreeMap, ops::Add};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypePathComponent<'a> {
    /// An index into an array
    Index(usize),

    /// A name of a struct (or union) field
    Name(&'a str),
}

/// Tracks the internal state of the type path parser
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TypePathParserState {
    /// The initial (empty) state
    Start,

    /// Inside a field name
    InsideName,

    /// Inside an index
    InsideIndex,

    /// After an index (expecting '.' or '[')
    AfterIndex,

    /// Expecting the first character of a name (after '.')
    ExpectingName,

    /// Parser has encountered an error
    Error,

    /// Parser is done
    Done,
}

/// An iterator over the components of a type path string
#[derive(Debug, Clone)]
struct TypePathComponentIter<'a> {
    path: &'a str,
    position: usize,
    state: TypePathParserState,
}

impl<'a> TypePathComponentIter<'a> {
    fn new(path: &'a str) -> Self {
        Self {
            path,
            position: 0,
            state: TypePathParserState::Start,
        }
    }
}

impl<'a> Iterator for TypePathComponentIter<'a> {
    type Item = BTFResult<TypePathComponent<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == TypePathParserState::Done || self.state == TypePathParserState::Error {
            return None;
        }

        let bytes = self.path.as_bytes();

        // Handle empty path or end of input in certain states
        if self.position >= bytes.len() {
            self.state = TypePathParserState::Done;
            return None;
        }

        match self.state {
            TypePathParserState::Start => {
                let c = bytes[self.position] as char;
                if c == '[' {
                    self.position += 1;
                    self.state = TypePathParserState::InsideIndex;
                    self.parse_index()
                } else if c.is_alphabetic() || c == '_' {
                    self.state = TypePathParserState::InsideName;
                    self.parse_name()
                } else {
                    self.state = TypePathParserState::Error;
                    Some(Err(BTFError::new(
                        BTFErrorKind::InvalidTypePath,
                        &format!("Invalid character at index {}", self.position),
                    )))
                }
            }

            TypePathParserState::InsideName => self.parse_name(),

            TypePathParserState::InsideIndex => self.parse_index(),

            TypePathParserState::AfterIndex => {
                let c = bytes[self.position] as char;
                if c == '[' {
                    self.position += 1;
                    self.state = TypePathParserState::InsideIndex;
                    self.parse_index()
                } else if c == '.' {
                    self.position += 1;
                    self.state = TypePathParserState::ExpectingName;
                    self.next()
                } else {
                    self.state = TypePathParserState::Error;
                    Some(Err(BTFError::new(
                        BTFErrorKind::InvalidTypePath,
                        &format!("Invalid character at index {}", self.position),
                    )))
                }
            }

            TypePathParserState::ExpectingName => {
                if self.position >= bytes.len() {
                    self.state = TypePathParserState::Error;
                    return Some(Err(BTFError::new(
                        BTFErrorKind::InvalidTypePath,
                        "Expected name after '.'",
                    )));
                }
                let c = bytes[self.position] as char;
                if c.is_alphabetic() || c == '_' {
                    self.state = TypePathParserState::InsideName;
                    self.parse_name()
                } else {
                    self.state = TypePathParserState::Error;
                    Some(Err(BTFError::new(
                        BTFErrorKind::InvalidTypePath,
                        &format!("Invalid character at index {}", self.position),
                    )))
                }
            }

            TypePathParserState::Done | TypePathParserState::Error => None,
        }
    }
}

impl<'a> TypePathComponentIter<'a> {
    fn parse_name(&mut self) -> Option<BTFResult<TypePathComponent<'a>>> {
        let start = self.position;
        let bytes = self.path.as_bytes();

        while self.position < bytes.len() {
            let c = bytes[self.position] as char;
            if c.is_alphanumeric() || c == '_' {
                self.position += 1;
            } else if c == '[' || c == '.' {
                break;
            } else {
                self.state = TypePathParserState::Error;
                return Some(Err(BTFError::new(
                    BTFErrorKind::InvalidTypePath,
                    &format!("Invalid character at index {}", self.position),
                )));
            }
        }

        let name = &self.path[start..self.position];

        // Determine next state
        if self.position >= bytes.len() {
            self.state = TypePathParserState::Done;
        } else {
            let c = bytes[self.position] as char;
            if c == '[' {
                self.position += 1;
                self.state = TypePathParserState::InsideIndex;
            } else if c == '.' {
                self.position += 1;
                self.state = TypePathParserState::ExpectingName;
            }
        }

        Some(Ok(TypePathComponent::Name(name)))
    }

    fn parse_index(&mut self) -> Option<BTFResult<TypePathComponent<'a>>> {
        let start = self.position;
        let bytes = self.path.as_bytes();

        while self.position < bytes.len() {
            let c = bytes[self.position] as char;
            if c.is_numeric() {
                self.position += 1;
            } else if c == ']' {
                break;
            } else {
                self.state = TypePathParserState::Error;
                return Some(Err(BTFError::new(
                    BTFErrorKind::InvalidTypePath,
                    &format!("Invalid character at index {}", self.position),
                )));
            }
        }

        if self.position >= bytes.len() || bytes[self.position] as char != ']' {
            self.state = TypePathParserState::Error;
            return Some(Err(BTFError::new(
                BTFErrorKind::InvalidTypePath,
                "Unclosed index bracket",
            )));
        }

        let index_str = &self.path[start..self.position];
        if index_str.is_empty() {
            self.state = TypePathParserState::Error;
            return Some(Err(BTFError::new(
                BTFErrorKind::InvalidTypePath,
                "Empty index",
            )));
        }

        let index = match index_str.parse::<usize>() {
            Ok(i) => i,
            Err(error) => {
                self.state = TypePathParserState::Error;
                return Some(Err(BTFError::new(
                    BTFErrorKind::InvalidTypePath,
                    &format!("Invalid index value: {error:?}"),
                )));
            }
        };

        // Skip the ']'
        self.position += 1;
        self.state = TypePathParserState::AfterIndex;

        Some(Ok(TypePathComponent::Index(index)))
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

// Generate the parse_type functions for each type
generate_constructor_dispatcher!(
    Int, Typedef, Enum, Ptr, Const, Volatile, Array, FuncProto, Struct, Union, Fwd, Var, Enum64,
    Func, Float, Restrict, DataSec, TypeTag, DeclTag
);

/// Lightweight error type for offset_of_helper that avoids string formatting.
/// Only formatted into a full BTFError when the error escapes to the caller.
#[derive(Debug)]
enum OffsetError<'a> {
    InvalidTypeId,
    VoidDereference,
    IndexOutOfBounds { index: usize, array_size: u32 },
    ArrayOffsetOverflow,
    PtrNotIndexable,
    TypeNotIndexable,
    NotStructOrUnion,
    MemberNotFound { name: &'a str },
    /// Wraps an existing BTFError (e.g., from offset.add() or iterator)
    Btf(BTFError),
}

impl From<BTFError> for OffsetError<'_> {
    fn from(e: BTFError) -> Self {
        OffsetError::Btf(e)
    }
}

impl From<OffsetError<'_>> for BTFError {
    fn from(e: OffsetError<'_>) -> Self {
        match e {
            OffsetError::InvalidTypeId => {
                BTFError::new(BTFErrorKind::InvalidTypeID, "Invalid type id")
            }
            OffsetError::VoidDereference => BTFError::new(
                BTFErrorKind::InvalidTypePath,
                "The void type can't be dereferenced with a path",
            ),
            OffsetError::IndexOutOfBounds { index, array_size } => BTFError::new(
                BTFErrorKind::InvalidTypePath,
                &format!(
                    "Index {index} is out of bounds for array of size {array_size}"
                ),
            ),
            OffsetError::ArrayOffsetOverflow => {
                BTFError::new(BTFErrorKind::InvalidTypePath, "Array element offset overflow")
            }
            OffsetError::PtrNotIndexable => BTFError::new(
                BTFErrorKind::InvalidTypePath,
                "Type is a ptr, and dereferencing it would require a read operation",
            ),
            OffsetError::TypeNotIndexable => {
                BTFError::new(BTFErrorKind::InvalidTypePath, "Type is not indexable")
            }
            OffsetError::NotStructOrUnion => {
                BTFError::new(BTFErrorKind::InvalidTypePath, "Type is not a struct or union")
            }
            OffsetError::MemberNotFound { name } => BTFError::new(
                BTFErrorKind::InvalidTypePath,
                &format!("Member '{}' not found", name),
            ),
            OffsetError::Btf(e) => e,
        }
    }
}

impl TypeInformation {
    /// Creates a new `TypeInformation` object
    pub fn new(readable: &dyn Readable) -> BTFResult<Self> {
        let mut reader = Reader::new(readable);

        let file_header = FileHeader::new(&mut reader)?;
        let type_section_start = (file_header
            .hdr_len()
            .checked_add(file_header.type_off())
            .ok_or_else(|| {
                BTFError::new(
                    BTFErrorKind::InvalidTypeSectionOffset,
                    "Type section start offset overflow",
                )
            })?) as usize;

        let type_section_end = type_section_start
            .checked_add(file_header.type_len() as usize)
            .ok_or_else(|| {
                BTFError::new(
                    BTFErrorKind::InvalidTypeSectionOffset,
                    "Type section end offset overflow",
                )
            })?;

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

    /// Returns the pointee type id
    pub fn pointee_tid(&self, tid: u32) -> BTFResult<u32> {
        match self.from_id(tid) {
            None => Err(BTFError::new(
                BTFErrorKind::InvalidTypeID,
                "Invalid type id",
            )),

            Some(type_variant) => match type_variant {
                TypeVariant::Typedef(typedef) => self.pointee_tid(*typedef.tid()),
                TypeVariant::Const(cnst) => self.pointee_tid(*cnst.tid()),
                TypeVariant::Volatile(volatile) => self.pointee_tid(*volatile.tid()),
                TypeVariant::Restrict(restrict) => self.pointee_tid(*restrict.tid()),

                TypeVariant::Ptr(ptr) => Ok(*ptr.tid()),

                _ => Err(BTFError::new(
                    BTFErrorKind::InvalidTypeID,
                    "Type is not a pointer",
                )),
            },
        }
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
                &format!("Type {type_variant:?} has no size"),
            )),
        }
    }

    /// Returns a tuple containing the next type id and the current offset
    pub fn offset_of(&self, tid: u32, path: &str) -> BTFResult<(u32, Offset)> {
        let mut path_iter = TypePathComponentIter::new(path);
        self.offset_of_impl(Offset::ByteOffset(0), tid, &mut path_iter)
            .map_err(Into::into)
    }

    /// Internal helper method for `TypeInformation::offset_of`
    ///
    /// Returns a lightweight `OffsetError` that avoids string formatting.
    /// Errors are only formatted when converted to `BTFError` at the public API boundary.
    fn offset_of_impl<'a>(
        &self,
        mut offset: Offset,
        mut tid: u32,
        path: &mut TypePathComponentIter<'a>,
    ) -> Result<(u32, Offset), OffsetError<'a>> {
        loop {
            // Save iterator position before consuming (for anonymous member probing)
            let path_for_anon = path.clone();

            // Get next component
            let component = match path.next() {
                None => return Ok((tid, offset)),
                Some(result) => result?,
            };

            // Resolve through type indirections (Fwd, Typedef, Const, Volatile, Restrict)
            let type_var = loop {
                let type_var = self.from_id(tid).ok_or(OffsetError::InvalidTypeId)?;

                match &type_var {
                    TypeVariant::Fwd(fwd) => {
                        tid = *fwd.tid();
                    }
                    TypeVariant::Typedef(typedef) => {
                        tid = *typedef.tid();
                    }
                    TypeVariant::Const(cnst) => {
                        tid = *cnst.tid();
                    }
                    TypeVariant::Volatile(volatile) => {
                        tid = *volatile.tid();
                    }
                    TypeVariant::Restrict(restrict) => {
                        tid = *restrict.tid();
                    }
                    _ => break type_var,
                }
            };

            // Check for void
            if matches!(type_var, TypeVariant::Void) {
                return Err(OffsetError::VoidDereference);
            }

            match component {
                TypePathComponent::Index(index) => {
                    match &type_var {
                        TypeVariant::Array(array) => {
                            let element_count = *array.element_count() as usize;
                            if index >= element_count {
                                return Err(OffsetError::IndexOutOfBounds {
                                    index,
                                    array_size: *array.element_count(),
                                });
                            }

                            let element_tid = *array.element_tid();
                            let element_type_size = self.size_of(element_tid)?;

                            let index_size = (index as u64)
                                .checked_mul(element_type_size as u64)
                                .and_then(|v| u32::try_from(v).ok())
                                .ok_or(OffsetError::ArrayOffsetOverflow)?;

                            offset = offset.add(index_size)?;
                            tid = element_tid;
                        }

                        TypeVariant::Ptr(_) => {
                            return Err(OffsetError::PtrNotIndexable);
                        }

                        _ => {
                            return Err(OffsetError::TypeNotIndexable);
                        }
                    }
                }

                TypePathComponent::Name(name) => {
                    let member_list: &[_] = match &type_var {
                        TypeVariant::Struct(s) => s.member_list(),
                        TypeVariant::Union(u) => u.member_list(),
                        _ => {
                            return Err(OffsetError::NotStructOrUnion);
                        }
                    };

                    // Try anonymous members first (using path_for_anon which includes current component)
                    for member in member_list.iter().filter(|m| m.name().is_none()) {
                        let mut anon_path = path_for_anon.clone();
                        match self.offset_of_impl(
                            member.offset().add(offset)?,
                            member.tid(),
                            &mut anon_path,
                        ) {
                            Ok((result_tid, result_offset)) => {
                                return Ok((result_tid, result_offset));
                            }
                            Err(_) => continue,
                        }
                    }

                    // Try named members
                    match member_list.iter().find(|member| {
                        member.name().as_deref() == Some(name)
                    }) {
                        Some(member) => {
                            tid = member.tid();
                            offset = offset.add(member.offset())?;
                        }
                        None => {
                            return Err(OffsetError::MemberNotFound { name });
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::btf::{
        data_sec::Variable as DataSecVariable,
        enum64::{Integer64Value as IntegerValue64, NamedValue64},
        r#enum::{Integer32Value as IntegerValue32, NamedValue32},
        struct_union::Member as StructMember,
        LinkageType,
    };
    use crate::utils::ReadableBuffer;

    /// Helper to collect iterator results into a Vec for testing
    fn collect_path_components(path: &str) -> BTFResult<Vec<TypePathComponent<'_>>> {
        TypePathComponentIter::new(path).collect()
    }

    #[test]
    fn test_path_component_iter() {
        let type_path = collect_path_components("").unwrap();
        assert!(type_path.is_empty());

        let type_path = collect_path_components("[1]").unwrap();
        assert_eq!(type_path.len(), 1);
        assert_eq!(type_path[0], TypePathComponent::Index(1));

        let type_path = collect_path_components("[1][2]").unwrap();
        assert_eq!(type_path.len(), 2);
        assert_eq!(type_path[0], TypePathComponent::Index(1));
        assert_eq!(type_path[1], TypePathComponent::Index(2));

        let type_path = collect_path_components("test").unwrap();
        assert_eq!(type_path.len(), 1);
        assert_eq!(type_path[0], TypePathComponent::Name("test"));

        let type_path = collect_path_components("array[10]").unwrap();
        assert_eq!(type_path.len(), 2);
        assert_eq!(type_path[0], TypePathComponent::Name("array"));
        assert_eq!(type_path[1], TypePathComponent::Index(10));

        let type_path = collect_path_components("array[10].array2[11]").unwrap();
        assert_eq!(type_path.len(), 4);
        assert_eq!(type_path[0], TypePathComponent::Name("array"));
        assert_eq!(type_path[1], TypePathComponent::Index(10));
        assert_eq!(type_path[2], TypePathComponent::Name("array2"));
        assert_eq!(type_path[3], TypePathComponent::Index(11));

        // Test underscore in names
        let type_path = collect_path_components("_test").unwrap();
        assert_eq!(type_path.len(), 1);
        assert_eq!(type_path[0], TypePathComponent::Name("_test"));

        let type_path = collect_path_components("test_field").unwrap();
        assert_eq!(type_path.len(), 1);
        assert_eq!(type_path[0], TypePathComponent::Name("test_field"));

        assert!(collect_path_components(".value").is_err());
        assert!(collect_path_components(".[10]").is_err());
        assert!(collect_path_components("[value").is_err());
        assert!(collect_path_components("]value").is_err());
        assert!(collect_path_components("1").is_err());
        assert!(collect_path_components("array[10]value").is_err());
        assert!(collect_path_components("array[]").is_err());
        assert!(collect_path_components("[]").is_err());
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
                32,
            )),
        );

        type_info
            .name_to_id_map
            .insert(String::from("unsigned int"), 1);

        type_info
            .id_to_name_map
            .insert(1, String::from("unsigned int"));

        // tid:2 BTF_KIND_PTR. Make this reference the named struct type we define
        // later on
        type_info.id_to_type_map.insert(
            2,
            TypeVariant::Ptr(Ptr::create(Header::create(Kind::Ptr, 0, 0, false, 6), 6)),
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
                Header::create(Kind::Struct, 0, 2, false, 8),
                None,
                8,
                vec![
                    StructMember::create(
                        1,
                        Some(String::from("anon_struct_value1")),
                        1,
                        Offset::ByteOffset(0),
                    ),
                    StructMember::create(
                        1,
                        Some(String::from("anon_struct_value2")),
                        1,
                        Offset::ByteOffset(32),
                    ),
                ],
            )),
        );

        // tid:5 Anonymous union type
        type_info.id_to_type_map.insert(
            5,
            TypeVariant::Union(Union::create(
                Header::create(Kind::Union, 0, 2, true, 8),
                None,
                8,
                vec![
                    StructMember::create(
                        1,
                        Some(String::from("anon_union_value1")),
                        1,
                        Offset::ByteOffset(0),
                    ),
                    StructMember::create(
                        1,
                        Some(String::from("anon_union_value2")),
                        2,
                        Offset::ByteOffset(0),
                    ),
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
                    StructMember::create(0, None, 4, Offset::ByteOffset(0)),
                    StructMember::create(0, None, 5, Offset::ByteOffset(64)),
                    StructMember::create(
                        1,
                        Some(String::from("int_value")),
                        1,
                        Offset::ByteOffset(128),
                    ),
                    StructMember::create(
                        1,
                        Some(String::from("ptr_value")),
                        2,
                        Offset::ByteOffset(160),
                    ),
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
                    StructMember::create(1, Some(String::from("next")), 2, Offset::ByteOffset(0)),
                    StructMember::create(1, Some(String::from("prev")), 2, Offset::ByteOffset(64)),
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
                false,
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
                false,
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
                Header::create(Kind::Volatile, 0, 0, false, 6),
                6,
            )),
        );

        // tid:13 BTF_KIND_CONST
        type_info.id_to_type_map.insert(
            13,
            TypeVariant::Const(Const::create(
                Header::create(Kind::Const, 0, 0, false, 6),
                6,
            )),
        );

        // tid:14 BTF_KIND_RESTRICT
        type_info.id_to_type_map.insert(
            14,
            TypeVariant::Restrict(Restrict::create(
                Header::create(Kind::Restrict, 0, 0, false, 6),
                6,
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

        // Additional nested structs/unions scenario:
        //
        // struct qstr {
        //   union {
        //     struct {
        //       int hash;
        //       int len;
        //     }
        //
        //     unsigned long int hash_len;
        //   }
        //
        //   unsigned long int name;
        // }
        //
        // struct dentry {
        //   int test1;
        //   struct qstr d_name;
        //   int test2;
        // }

        type_info.id_to_type_map.insert(
            100,
            TypeVariant::Int(Int::create(
                Header::create(Kind::Int, 1, 0, false, 8),
                Some(String::from("unsigned long int")),
                8,
                false,
                false,
                false,
                0,
                64,
            )),
        );

        type_info.id_to_type_map.insert(
            101,
            TypeVariant::Struct(Struct::create(
                Header::create(Kind::Struct, 0, 2, false, 8),
                None,
                8,
                vec![
                    StructMember::create(1, Some(String::from("hash")), 1, Offset::ByteOffset(0)),
                    StructMember::create(1, Some(String::from("len")), 1, Offset::ByteOffset(4)),
                ],
            )),
        );

        type_info.id_to_type_map.insert(
            102,
            TypeVariant::Union(Union::create(
                Header::create(Kind::Union, 0, 2, false, 8),
                None,
                8,
                vec![
                    StructMember::create(0, None, 101, Offset::ByteOffset(0)),
                    StructMember::create(
                        1,
                        Some(String::from("hash_len")),
                        100,
                        Offset::ByteOffset(0),
                    ),
                ],
            )),
        );

        type_info.id_to_type_map.insert(
            103,
            TypeVariant::Struct(Struct::create(
                Header::create(Kind::Struct, 1, 2, false, 8),
                Some(String::from("qstr")),
                8,
                vec![
                    StructMember::create(0, None, 102, Offset::ByteOffset(0)),
                    StructMember::create(1, Some(String::from("name")), 100, Offset::ByteOffset(8)),
                ],
            )),
        );

        type_info.id_to_type_map.insert(
            104,
            TypeVariant::Struct(Struct::create(
                Header::create(Kind::Struct, 1, 3, false, 16),
                Some(String::from("dentry")),
                8,
                vec![
                    StructMember::create(1, Some(String::from("test1")), 1, Offset::ByteOffset(0)),
                    StructMember::create(
                        1,
                        Some(String::from("d_name")),
                        103,
                        Offset::ByteOffset(32),
                    ),
                    StructMember::create(
                        1,
                        Some(String::from("test2")),
                        1,
                        Offset::ByteOffset(256),
                    ),
                ],
            )),
        );

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

        // The typedef references the named struct, which is 28 bytes
        assert_eq!(type_info.size_of(11).unwrap(), 28);

        // The volatile references the named struct `Struct`, which is 28 bytes
        assert_eq!(type_info.size_of(12).unwrap(), 28);

        // The const references the named struct `Struct`, which is 28 bytes
        assert_eq!(type_info.size_of(13).unwrap(), 28);

        // The restrict references the named struct `Struct`, which is 28 bytes
        assert_eq!(type_info.size_of(14).unwrap(), 28);

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

        // The BTF_KIND_TYPE_TAG has size of the type it is applied to (named struct)
        assert_eq!(type_info.size_of(21).unwrap(), 28);
    }

    #[test]
    fn pointee_tid() {
        let type_info = get_test_type_info();
        assert_eq!(type_info.pointee_tid(2).unwrap(), 6);
    }

    #[test]
    fn test_offset_of() {
        let type_info = get_test_type_info();

        // void, int, ptr, enum, enum64, fwd, float
        let no_deref_tid_list1 = [0, 1, 2, 8, 9, 10, 19];

        // func, func_proto, var, datasec, decl_tag, type tag
        let no_deref_tid_list2 = [15, 16, 17, 18, 20, 21];

        for tid in no_deref_tid_list1.iter().chain(no_deref_tid_list2.iter()) {
            assert_eq!(
                type_info.offset_of(*tid, "test").unwrap_err().kind(),
                BTFErrorKind::InvalidTypePath
            );

            assert_eq!(
                type_info.offset_of(*tid, "[0]").unwrap_err().kind(),
                BTFErrorKind::InvalidTypePath
            );
        }

        // Test invalid indexes
        assert_eq!(
            type_info
                .offset_of(3, "[10000000000000000000000000000]")
                .unwrap_err()
                .kind(),
            BTFErrorKind::InvalidTypePath
        );

        assert_eq!(
            type_info.offset_of(3, "[test]").unwrap_err().kind(),
            BTFErrorKind::InvalidTypePath
        );

        assert_eq!(
            type_info.offset_of(3, "test").unwrap_err().kind(),
            BTFErrorKind::InvalidTypePath
        );

        // Test valid indexes
        assert_eq!(
            type_info.offset_of(3, "[0]").unwrap(),
            (1, Offset::ByteOffset(0))
        );

        assert_eq!(
            type_info.offset_of(3, "[1]").unwrap(),
            (1, Offset::ByteOffset(4))
        );

        assert_eq!(
            type_info.offset_of(3, "[2]").unwrap(),
            (1, Offset::ByteOffset(8))
        );

        assert_eq!(
            type_info.offset_of(3, "[3]").unwrap(),
            (1, Offset::ByteOffset(12))
        );

        assert_eq!(
            type_info.offset_of(3, "[4]").unwrap(),
            (1, Offset::ByteOffset(16))
        );

        assert_eq!(
            type_info.offset_of(3, "[5]").unwrap(),
            (1, Offset::ByteOffset(20))
        );

        assert_eq!(
            type_info.offset_of(3, "[6]").unwrap(),
            (1, Offset::ByteOffset(24))
        );

        assert_eq!(
            type_info.offset_of(3, "[7]").unwrap(),
            (1, Offset::ByteOffset(28))
        );

        assert_eq!(
            type_info.offset_of(3, "[8]").unwrap(),
            (1, Offset::ByteOffset(32))
        );

        assert_eq!(
            type_info.offset_of(3, "[9]").unwrap(),
            (1, Offset::ByteOffset(36))
        );

        assert_eq!(
            type_info.offset_of(3, "[10]").unwrap_err().kind(),
            BTFErrorKind::InvalidTypePath
        );

        // Named struct and the typedef/const/volatile/restrict that reference it
        for struct_tid in [6, 11, 12, 13, 14] {
            let int_value_offset = type_info.offset_of(struct_tid, "int_value").unwrap();
            assert_eq!(int_value_offset, (1, Offset::ByteOffset(16 * 8)));

            let ptr_value_offset = type_info.offset_of(struct_tid, "ptr_value").unwrap();

            assert_eq!(ptr_value_offset, (2, Offset::ByteOffset(20 * 8)));

            let anon_struct_value1_offset = type_info
                .offset_of(struct_tid, "anon_struct_value1")
                .unwrap();

            assert_eq!(anon_struct_value1_offset, (1, Offset::ByteOffset(0)));

            let anon_struct_value2_offset = type_info
                .offset_of(struct_tid, "anon_struct_value2")
                .unwrap();

            assert_eq!(anon_struct_value2_offset, (1, Offset::ByteOffset(4 * 8)));

            let anon_union_value1_offset = type_info
                .offset_of(struct_tid, "anon_union_value1")
                .unwrap();

            assert_eq!(anon_union_value1_offset, (1, Offset::ByteOffset(8 * 8)));

            let anon_union_value2_offset = type_info
                .offset_of(struct_tid, "anon_union_value2")
                .unwrap();

            assert_eq!(anon_union_value2_offset, (2, Offset::ByteOffset(8 * 8)));
        }

        assert_eq!(
            type_info.offset_of(104, "test1").unwrap(),
            (1, Offset::ByteOffset(0))
        );

        assert_eq!(
            type_info.offset_of(104, "d_name").unwrap(),
            (103, Offset::ByteOffset(32))
        );

        assert_eq!(
            type_info.offset_of(104, "test2").unwrap(),
            (1, Offset::ByteOffset(256))
        );

        assert_eq!(
            type_info.offset_of(104, "d_name.hash").unwrap(),
            (1, Offset::ByteOffset(32))
        );

        assert_eq!(
            type_info.offset_of(104, "d_name.len").unwrap(),
            (1, Offset::ByteOffset(36))
        );

        assert_eq!(
            type_info.offset_of(104, "d_name.hash_len").unwrap(),
            (100, Offset::ByteOffset(32))
        );

        assert_eq!(
            type_info.offset_of(104, "d_name.name").unwrap(),
            (100, Offset::ByteOffset(40))
        );
    }

    #[test]
    fn test_type_section_offset_overflow() {
        // Test overflow when calculating type section start (hdr_len + type_off)
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0xFF, 0xFF, 0xFF, 0x7F, // hdr_len
            0xFF, 0xFF, 0xFF, 0x7F, // type_off
            0x01, 0x00, 0x00, 0x00, // type_len
            0x00, 0x00, 0x00, 0x00, // str_off
            0x01, 0x00, 0x00, 0x00, // str_len
        ]);

        let result = TypeInformation::new(&readable_buffer);
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err.kind(), BTFErrorKind::InvalidTypeSectionOffset);
        }
    }

    #[test]
    fn test_type_section_end_overflow() {
        // Test overflow when calculating type section end (start + type_len)
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0xFF, // type_off
            0xFF, 0xFF, 0xFF, 0x01, // type_len
            0x00, 0x00, 0x00, 0x00, // str_off
            0x01, 0x00, 0x00, 0x00, // str_len
        ]);

        let result = TypeInformation::new(&readable_buffer);
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err.kind(), BTFErrorKind::InvalidTypeSectionOffset);
        }
    }

    #[test]
    fn test_array_element_offset_overflow() {
        // Create an array with very large element size that will overflow
        // when calculating offset at high indexes
        let mut type_info = get_test_type_info();

        // tid:200 - Large int type (size close to u32::MAX / small divisor)
        type_info.id_to_type_map.insert(
            200,
            TypeVariant::Int(Int::create(
                Header::create(Kind::Int, 1, 0, false, 0x10000000),
                Some(String::from("huge_int")),
                0x10000000,
                false,
                false,
                false,
                0,
                32,
            )),
        );

        // tid:201 - Array of 100 huge_ints
        type_info.id_to_type_map.insert(
            201,
            TypeVariant::Array(Array::create(
                Header::create(Kind::Array, 0, 0, false, 0),
                200, // element type
                200, // index type
                100, // element count
            )),
        );

        let result = type_info.offset_of(201, "[20]");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), BTFErrorKind::InvalidTypePath);

        let result = type_info.offset_of(201, "[0]");
        assert!(result.is_ok());
    }
}
