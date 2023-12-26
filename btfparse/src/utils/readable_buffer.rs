use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Readable, Result as BTFResult};

pub struct ReadableBuffer<'a> {
    buffer: &'a [u8],
}

impl<'a> ReadableBuffer<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer }
    }
}

impl<'a> Readable for ReadableBuffer<'a> {
    fn read(&self, offset: u64, buffer: &mut [u8]) -> BTFResult<()> {
        let source_start_offset = offset as usize;
        if source_start_offset == self.buffer.len() {
            return Err(BTFError::new(
                BTFErrorKind::EOF,
                "There are no bytes left to read",
            ));
        }

        let source_end_offset = source_start_offset + buffer.len();
        match source_end_offset.cmp(&self.buffer.len()) {
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                let source_slice = &self.buffer[source_start_offset..source_end_offset];
                buffer.copy_from_slice(source_slice);

                Ok(())
            }

            std::cmp::Ordering::Greater => Err(BTFError::new(
                BTFErrorKind::InvalidOffset,
                "There are not enough bytes left to complete the read request",
            )),
        }
    }
}
