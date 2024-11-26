/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::btf::{Error as BTFError, ErrorKind as BTFErrorKind, FileHeader, Result as BTFResult};
use crate::utils::Reader;

/// Returns the string at offset `string_offset`
pub fn parse_string(
    reader: &mut Reader,
    file_header: &FileHeader,
    string_offset: u32,
) -> BTFResult<String> {
    let string_section_start = file_header.hdr_len() + file_header.str_off();
    let string_section_end = string_section_start + file_header.str_len();

    let string_offset = string_section_start + string_offset;
    if string_offset >= string_section_end {
        return Err(BTFError::new(
            BTFErrorKind::InvalidStringOffset,
            &format!("Invalid string offset 0x{:08X}", string_offset),
        ));
    }

    let original_offset = reader.offset();
    reader.set_offset(string_offset as usize);

    let mut string = String::new();
    loop {
        if reader.offset() + 1 > string_section_end as usize {
            return Err(BTFError::new(
                BTFErrorKind::InvalidString,
                &format!(
                    "String at offset 0x{:08X} is not correctly null terminated",
                    string_offset
                ),
            ));
        }

        let character = reader.u8().inspect_err(|_error| {
            // Restore the original offset in case of error
            reader.set_offset(original_offset);
        })? as char;

        if character == '\0' {
            break;
        }

        string.push(character);
    }

    reader.set_offset(original_offset);
    Ok(string)
}

#[cfg(test)]
mod tests {
    use super::{parse_string, FileHeader};
    use crate::utils::{ReadableBuffer, Reader};

    #[test]
    fn test_parse_string() {
        let readable_buffer = ReadableBuffer::new(&[
            //
            // BTF header
            //
            0x9F, 0xEB, // magic
            0x01, // version
            0x00, // flags
            0x18, 0x00, 0x00, 0x00, // hdr_len
            0x00, 0x00, 0x00, 0x00, // type_off
            0x01, 0x00, 0x00, 0x00, // type_len
            0x01, 0x00, 0x00, 0x00, // str_off
            0x0B, 0x00, 0x00, 0x00, // str_len
            //
            // Type section
            //
            0x00,
            //
            // String section
            //
            0x00, // Null string (must be present)
            0x41, 0x42, 0x43, 0x44, 0x00, // ABCD\0
            0x45, 0x46, 0x47, 0x48, 0x00, // EFGH\0
        ]);

        let mut reader = Reader::new(&readable_buffer);
        let file_header = FileHeader::new(&mut reader).unwrap();

        let null_string = parse_string(&mut reader, &file_header, 0).unwrap();
        assert!(null_string.is_empty());

        let valid_string = parse_string(&mut reader, &file_header, 1).unwrap();
        assert_eq!(valid_string, "ABCD");

        let valid_string = parse_string(&mut reader, &file_header, 6).unwrap();
        assert_eq!(valid_string, "EFGH");

        assert!(parse_string(&mut reader, &file_header, 11).is_err());
    }
}
