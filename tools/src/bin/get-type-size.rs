use std::{env, fs::File, os::unix::fs::FileExt, path::Path};

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
    let argument_list: Vec<String> = env::args().collect();
    if argument_list.len() != 3 {
        println!("Usage:\n\tget-type-size /path/to/btf/file <type_name>\n");
        return;
    }

    let btf_file_path = Path::new(&argument_list[1]);
    let btf_type_name = &argument_list[2];

    println!("Opening BTF file: {:?}", btf_file_path);

    let vmlinux_btf_file = ReadableFile::new(btf_file_path);
    let type_information = TypeInformation::new(&vmlinux_btf_file).unwrap();

    let type_id = type_information.type_id(btf_type_name).unwrap();
    let type_size = type_information.type_size(type_id).unwrap();
    println!(
        "Type {} has ID {} and requires {} bytes in total",
        btf_type_name, type_id, type_size
    );
}
