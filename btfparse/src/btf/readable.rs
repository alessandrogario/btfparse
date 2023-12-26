use crate::btf::Result as BTFResult;

/// A trait for reading bytes from a source
pub trait Readable {
    /// Reads `buffer.len()` bytes from the given offset
    fn read(&self, offset: u64, buffer: &mut [u8]) -> BTFResult<()>;
}
