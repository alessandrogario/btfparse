# btfparse

btfparse is a library that can be used to parse the [BPF Type Format (BTF)](https://docs.kernel.org/bpf/btf.html)

# Examples


## Retrieving member offsets

The full source code for this example can be found in the `./src/bin/get-type-offset.rs` file.

```rust
fn main() {
    let argument_list: Vec<String> = env::args().collect();
    if argument_list.len() != 4 {
        println!("Usage:\n\tget-type-offset /path/to/btf/file <type_name> <path>\n");
        return;
    }

    let btf_file_path = Path::new(&argument_list[1]);
    let btf_type_name = &argument_list[2];
    let type_path = &argument_list[3];

    println!("Opening BTF file: {:?}", btf_file_path);

    let vmlinux_btf_file = ReadableFile::new(btf_file_path);
    let type_information = TypeInformation::new(&vmlinux_btf_file).unwrap();
    let offset = type_information
        .offset_of(type_information.id_of(btf_type_name).unwrap(), type_path)
        .unwrap();

    println!("{} => {}: {:?}", btf_type_name, type_path, offset);
}
```

Example output:

```bash
sh-5.2$ get-type-offset /sys/kernel/btf/vmlinux 'dentry' 'd_name.len'
Opening BTF file: "/sys/kernel/btf/vmlinux"
dentry => d_name.len: (23, ByteOffset(36))
``````
