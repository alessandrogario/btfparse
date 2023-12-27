use std::{fs::File, os::unix::fs::FileExt, path::Path};

use btfparse::{Readable, Result as BTFResult, TypeInformation};

struct ReadableFile {
    file: File,
}

impl ReadableFile {
    fn new(path: &Path) -> Self {
        ReadableFile {
            file: File::open(path).unwrap(),
        }
    }
}

impl Readable for ReadableFile {
    fn read(&self, offset: u64, buffer: &mut [u8]) -> BTFResult<()> {
        self.file
            .read_exact_at(buffer, offset)
            .map_err(|err| err.into())
    }
}

fn main() {
    let vmlinux_btf_file = ReadableFile::new(Path::new("/sys/kernel/btf/vmlinux"));
    let _type_information = TypeInformation::new(&vmlinux_btf_file).unwrap();
}
