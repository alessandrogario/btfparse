use crate::btf::Header;

/// Common methods for all BTF types
pub trait Type {
    /// Returns the type header
    fn header(&self) -> &Header;
}
