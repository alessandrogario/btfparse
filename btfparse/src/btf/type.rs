use crate::btf::Kind;

/// Common methods for all BTF types
pub trait Type {
    /// Returns the name of the type
    fn name(&self) -> Option<String>;

    /// Returns the offset, in the string section, of the type name
    fn name_offset(&self) -> u32;

    /// Returns the `vlen` field of the type header
    fn vlen(&self) -> usize;

    /// Returns the `kind` field of the type header
    fn kind(&self) -> Kind;

    /// Returns the `kind_flag` field of the type header
    fn kind_flag(&self) -> bool;

    /// Returns the `size_or_type` field of the type header
    fn size_or_type(&self) -> u32;
}
