use crate::btf::{Readable, Result as BTFResult};

/// Endianness type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Endianness {
    /// Little endian
    Little,

    /// Big endian
    Big,
}

/// A reader class, to be used with a `Readable` source
pub struct Reader<'a> {
    /// The `Readable` source to read from
    readable: &'a dyn Readable,

    /// The current offset
    offset: usize,

    /// The current endianness
    endianness: Endianness,
}

impl<'a> Reader<'a> {
    /// Creates a new `Reader` instance
    pub fn new(readable: &'a dyn Readable) -> Reader {
        Reader {
            readable,
            offset: 0,
            endianness: Endianness::Little,
        }
    }

    /// Returns the current endianness
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Sets the current endianness
    pub fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness
    }

    /// Returns the current offset
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Sets the current offset
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Reads `buffer.len()` bytes from the current offset
    pub fn read(&mut self, buffer: &mut [u8]) -> BTFResult<()> {
        self.readable.read(self.offset as u64, buffer)?;
        self.offset += buffer.len();

        Ok(())
    }

    /// Reads a `u8` from the current offset
    pub fn u8(&mut self) -> BTFResult<u8> {
        let mut buffer: [u8; 1] = [0; 1];
        self.read(&mut buffer)?;

        Ok(buffer[0])
    }

    /// Reads a `u16` from the current offset
    pub fn u16(&mut self) -> BTFResult<u16> {
        let mut buffer: [u8; 2] = [0; 2];
        self.read(&mut buffer)?;

        let value = if self.endianness == Endianness::Little {
            u16::from_le_bytes(buffer)
        } else {
            u16::from_be_bytes(buffer)
        };

        Ok(value)
    }

    /// Reads a `u32` from the current offset
    pub fn u32(&mut self) -> BTFResult<u32> {
        let mut buffer: [u8; 4] = [0; 4];
        self.read(&mut buffer)?;

        let value = if self.endianness == Endianness::Little {
            u32::from_le_bytes(buffer)
        } else {
            u32::from_be_bytes(buffer)
        };

        Ok(value)
    }

    /// Reads a `u64` from the current offset
    pub fn u64(&mut self) -> BTFResult<u64> {
        let mut buffer: [u8; 8] = [0; 8];
        self.read(&mut buffer)?;

        let value = if self.endianness == Endianness::Little {
            u64::from_le_bytes(buffer)
        } else {
            u64::from_be_bytes(buffer)
        };

        Ok(value)
    }

    /// Reads an `i8` from the current offset
    pub fn i8(&mut self) -> BTFResult<i8> {
        self.u8().map(|value| value as i8)
    }

    /// Reads an `i16` from the current offset
    pub fn i16(&mut self) -> BTFResult<i16> {
        self.u16().map(|value| value as i16)
    }

    /// Reads an `i32` from the current offset
    pub fn i32(&mut self) -> BTFResult<i32> {
        self.u32().map(|value| value as i32)
    }

    /// Reads an `i64` from the current offset
    pub fn i64(&mut self) -> BTFResult<i64> {
        self.u64().map(|value| value as i64)
    }
}

#[cfg(test)]
mod tests {

    use crate::btf::ErrorKind as BTFErrorKind;
    use crate::utils::{Endianness, ReadableBuffer, Reader};

    macro_rules! check_error {
        ($reader:ident, $method:ident, $expected_error_kind:expr) => {
            let initial_offset = $reader.offset();

            let result = $reader.$method();
            assert!(result.is_err());

            let error = result.unwrap_err();
            assert_eq!(error.kind(), $expected_error_kind);

            let current_offset = $reader.offset();
            assert_eq!(current_offset, initial_offset);
        };
    }

    #[test]
    fn signed_values() {
        let buffer: [u8; 8] = [0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF];

        let readable_buffer = ReadableBuffer::new(&buffer);
        let mut reader = Reader::new(&readable_buffer);

        reader.set_endianness(Endianness::Little);

        reader.set_offset(0);
        assert_eq!(reader.i8().unwrap(), -8);

        reader.set_offset(0);
        assert_eq!(reader.i16().unwrap(), -1544);

        reader.set_offset(0);
        assert_eq!(reader.i32().unwrap(), -67438088);

        reader.set_offset(0);
        assert_eq!(reader.i64().unwrap(), -283686952306184);

        reader.set_endianness(Endianness::Big);

        reader.set_offset(0);
        assert_eq!(reader.i8().unwrap(), -8);

        reader.set_offset(0);
        assert_eq!(reader.i16().unwrap(), -1799);

        reader.set_offset(0);
        assert_eq!(reader.i32().unwrap(), -117835013);

        reader.set_offset(0);
        assert_eq!(reader.i64().unwrap(), -506097522914230529);
    }

    #[test]
    fn endianness() {
        let buffer: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        let readable_buffer = ReadableBuffer::new(&buffer);
        let mut reader = Reader::new(&readable_buffer);

        reader.set_endianness(Endianness::Little);

        reader.set_offset(0);
        assert_eq!(reader.u8().unwrap(), 0x01);

        reader.set_offset(0);
        assert_eq!(reader.u16().unwrap(), 0x0201);

        reader.set_offset(0);
        assert_eq!(reader.u32().unwrap(), 0x04030201);

        reader.set_offset(0);
        assert_eq!(reader.u64().unwrap(), 0x0807060504030201);

        reader.set_endianness(Endianness::Big);

        reader.set_offset(0);
        assert_eq!(reader.u8().unwrap(), 0x01);

        reader.set_offset(0);
        assert_eq!(reader.u16().unwrap(), 0x0102);

        reader.set_offset(0);
        assert_eq!(reader.u32().unwrap(), 0x01020304);

        reader.set_offset(0);
        assert_eq!(reader.u64().unwrap(), 0x0102030405060708);
    }

    #[test]
    fn eof() {
        let buffer: [u8; 0] = [];
        let readable_buffer = ReadableBuffer::new(&buffer);
        let mut reader = Reader::new(&readable_buffer);

        check_error!(reader, u8, BTFErrorKind::EOF);
        check_error!(reader, u16, BTFErrorKind::EOF);
        check_error!(reader, u32, BTFErrorKind::EOF);
        check_error!(reader, u64, BTFErrorKind::EOF);

        check_error!(reader, i8, BTFErrorKind::EOF);
        check_error!(reader, i16, BTFErrorKind::EOF);
        check_error!(reader, i32, BTFErrorKind::EOF);
        check_error!(reader, i64, BTFErrorKind::EOF);
    }

    #[test]
    fn invalid_offset() {
        let buffer: [u8; 1] = [1];
        let readable_buffer = ReadableBuffer::new(&buffer);
        let mut reader = Reader::new(&readable_buffer);

        check_error!(reader, u16, BTFErrorKind::InvalidOffset);
        check_error!(reader, u32, BTFErrorKind::InvalidOffset);
        check_error!(reader, u64, BTFErrorKind::InvalidOffset);

        check_error!(reader, i16, BTFErrorKind::InvalidOffset);
        check_error!(reader, i32, BTFErrorKind::InvalidOffset);
        check_error!(reader, i64, BTFErrorKind::InvalidOffset);
    }

    #[test]
    fn offset_increment() {
        let buffer: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let readable_buffer = ReadableBuffer::new(&buffer);
        let mut reader = Reader::new(&readable_buffer);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.u8().is_ok());
        assert_eq!(reader.offset(), 1);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.u16().is_ok());
        assert_eq!(reader.offset(), 2);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.u32().is_ok());
        assert_eq!(reader.offset(), 4);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.u64().is_ok());
        assert_eq!(reader.offset(), 8);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.i8().is_ok());
        assert_eq!(reader.offset(), 1);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.i16().is_ok());
        assert_eq!(reader.offset(), 2);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.i32().is_ok());
        assert_eq!(reader.offset(), 4);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        assert!(reader.i64().is_ok());
        assert_eq!(reader.offset(), 8);

        reader.set_offset(0);
        assert_eq!(reader.offset(), 0);
        let mut dest_buffer: [u8; 1] = [0];
        assert!(reader.read(&mut dest_buffer).is_ok());
        assert_eq!(reader.offset(), 1);
    }
}
