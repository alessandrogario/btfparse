/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

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
    if argument_list.len() != 2 {
        println!("Usage:\n\tdump-btf /path/to/btf/file\n");
        return;
    }

    let btf_file_path = Path::new(&argument_list[1]);
    println!("Opening BTF file: {btf_file_path:?}");

    let vmlinux_btf_file = ReadableFile::new(btf_file_path);
    let type_information = TypeInformation::new(&vmlinux_btf_file).unwrap();
    println!("{:?}", type_information.get());
}
