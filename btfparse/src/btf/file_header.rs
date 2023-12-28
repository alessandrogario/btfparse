use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, Result as BTFResult};
use crate::utils::{Endianness as ReaderEndianness, Reader};

/// BTF magic number (little endian)
const BTF_LITTLE_ENDIAN_MAGIC: u16 = 0xEB9F;

/// BTF magic number (big endian)
const BTF_BIG_ENDIAN_MAGIC: u16 = 0x9FEB;

/// BTF header
pub struct FileHeader {
    /// BTF version
    version: u8,

    /// BTF flags
    flags: u8,

    /// Header length
    hdr_len: u32,

    /// Offset of the type section
    type_off: u32,

    /// Length of the type section
    type_len: u32,

    /// Offset of the string section
    str_off: u32,

    /// Length of the string section
    str_len: u32,
}

impl FileHeader {
    /// Creates a new `FileHeader` instance
    pub fn new(reader: &mut Reader) -> BTFResult<Self> {
        reader.set_offset(0);
        Self::detect_endianness(reader)?;

        Ok(FileHeader {
            version: reader.u8()?,
            flags: reader.u8()?,
            hdr_len: reader.u32()?,
            type_off: reader.u32()?,
            type_len: reader.u32()?,
            str_off: reader.u32()?,
            str_len: reader.u32()?,
        })
    }

    /// Returns the BTF version
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Returns the BTF flags
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Returns the BTF header length
    pub fn hdr_len(&self) -> u32 {
        self.hdr_len
    }

    /// Returns the type section offset
    pub fn type_off(&self) -> u32 {
        self.type_off
    }

    /// Returns the type section length
    pub fn type_len(&self) -> u32 {
        self.type_len
    }

    /// Returns the string section offset
    pub fn str_off(&self) -> u32 {
        self.str_off
    }

    /// Returns the string section length
    pub fn str_len(&self) -> u32 {
        self.str_len
    }

    /// Detects the endianness of the BTF data
    fn detect_endianness(reader: &mut Reader) -> BTFResult<()> {
        match reader.u16()? {
            BTF_LITTLE_ENDIAN_MAGIC => {
                reader.set_endianness(ReaderEndianness::Little);
                Ok(())
            }

            BTF_BIG_ENDIAN_MAGIC => {
                reader.set_endianness(ReaderEndianness::Big);
                Ok(())
            }

            magic_value => Err(BTFError::new(
                BTFErrorKind::InvalidMagic,
                &format!("Invalid magic number: 0x{:04X}", magic_value),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileHeader;
    use crate::utils::{Endianness, ReadableBuffer, Reader};

    #[test]
    fn test_little_endian_btf_header() {
        let readable_buffer = ReadableBuffer::new(&[
            0x9F, 0xEB, // magic
            0x01, // version
            0x02, // flags
            0x03, 0x00, 0x00, 0x00, // hdr_len
            0x04, 0x00, 0x00, 0x00, // type_off
            0x05, 0x00, 0x00, 0x00, // type_len
            0x06, 0x00, 0x00, 0x00, // str_off
            0x07, 0x00, 0x00, 0x00, // str_len
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = FileHeader::new(&mut reader).unwrap();

        assert_eq!(reader.endianness(), Endianness::Little);
        assert_eq!(btf_header.version(), 1);
        assert_eq!(btf_header.flags(), 2);
        assert_eq!(btf_header.hdr_len(), 3);
        assert_eq!(btf_header.type_off(), 4);
        assert_eq!(btf_header.type_len(), 5);
        assert_eq!(btf_header.str_off(), 6);
        assert_eq!(btf_header.str_len(), 7);
    }

    #[test]
    fn test_big_endian_btf_header() {
        let readable_buffer = ReadableBuffer::new(&[
            0xEB, 0x9F, // magic
            0x01, // version
            0x02, // flags
            0x00, 0x00, 0x00, 0x03, // hdr_len
            0x00, 0x00, 0x00, 0x04, // type_off
            0x00, 0x00, 0x00, 0x05, // type_len
            0x00, 0x00, 0x00, 0x06, // str_off
            0x00, 0x00, 0x00, 0x07, // str_len
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let btf_header = FileHeader::new(&mut reader).unwrap();

        assert_eq!(reader.endianness(), Endianness::Big);
        assert_eq!(btf_header.version(), 1);
        assert_eq!(btf_header.flags(), 2);
        assert_eq!(btf_header.hdr_len(), 3);
        assert_eq!(btf_header.type_off(), 4);
        assert_eq!(btf_header.type_len(), 5);
        assert_eq!(btf_header.str_off(), 6);
        assert_eq!(btf_header.str_len(), 7);
    }
}
