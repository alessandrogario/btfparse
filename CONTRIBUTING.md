# Contributing to btfparse

Thank you for your interest in contributing to btfparse! We welcome contributions from the community and appreciate your effort to help improve this project.

## License Agreement

**By opening a pull request, you agree to license your contributions under the Apache License 2.0.**

This project is licensed under the [Apache License 2.0](LICENSE). When you submit code changes, your submissions are understood to be under the same Apache 2.0 License that covers the project, as defined in Section 5 of the Apache License 2.0:

> Unless You explicitly state otherwise, any Contribution intentionally submitted for inclusion in the Work by You to the Licensor shall be under the terms and conditions of this License, without any additional terms or conditions.

**IMPORTANT: We only accept contributions licensed under Apache 2.0. If you cannot license your work under Apache 2.0 (for example, due to employer restrictions or other licensing constraints), we cannot accept your contribution. Please do not submit pull requests for code that cannot be licensed under Apache 2.0.**

## Table of Contents

- [License Agreement](#license-agreement)
- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Enhancements](#suggesting-enhancements)
  - [Contributing Code](#contributing-code)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

This project adheres to a code of professional conduct. By participating, you are expected to uphold this standard. Please be respectful and constructive in all interactions.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Set up your development environment (see [Development Setup](#development-setup))
4. Create a new branch for your contribution
5. Make your changes
6. Submit a pull request

## How to Contribute

### Reporting Bugs

If you find a bug, please create an issue on GitHub with the following information:

- **Clear title**: A concise description of the problem
- **Description**: Detailed explanation of the issue
- **Steps to reproduce**: Minimal code or steps that demonstrate the bug
- **Expected behavior**: What you expected to happen
- **Actual behavior**: What actually happened
- **Environment**: Rust version, OS, btfparse version
- **Additional context**: Stack traces, error messages, or related issues

**Example bug report:**
```
Title: Parser fails on BTF files with DECL_TAG entries

Description:
When parsing BTF files that contain DECL_TAG type entries, the parser
panics with an index out of bounds error.

Steps to reproduce:
1. Load a BTF file with DECL_TAG entries using TypeInformation::new()
2. Call offset_of() on any type
3. Observe the panic

Expected: Should parse DECL_TAG entries correctly
Actual: Panics with "index out of bounds"

Environment:
- Rust 1.85.0
- btfparse 1.3.8
- Linux 6.17 with kernel BTF support
```

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

- **Use case**: Describe the problem you're trying to solve
- **Proposed solution**: Your suggested approach
- **Alternatives considered**: Other approaches you've thought about
- **Additional context**: Examples from other projects or use cases

**Example enhancement suggestion:**
```
Title: Add support for accessing array element types

Use case:
When working with BTF array types, I need to determine the element type
without manually traversing the type tree.

Proposed solution:
Add a TypeInformation::element_type_of(type_id) method that returns the
element type ID for array types.

Alternatives:
- Manual traversal using get_by_id() (current workaround)
- Adding an ArrayType helper struct
```

### Contributing Code

We welcome code contributions! Please consider the following:

- **Start small**: If you're new to the project, look for issues labeled "good first issue"
- **Discuss large changes**: For significant changes, please open an issue first to discuss the approach
- **Follow coding standards**: Ensure your code adheres to our [Coding Standards](#coding-standards)
- **Add tests**: Include tests for new functionality or bug fixes
- **Update documentation**: Update relevant documentation, including doc comments and README if needed

## Development Setup

### Prerequisites

- Rust 1.85 or later (edition 2024)
- A BTF file for testing (typically `/sys/kernel/btf/vmlinux` on Linux)

### Building

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/btfparse.git
cd btfparse

# Build the project
cargo build

# Run tests
cargo test

# Build with all features
cargo build --all-features
```

### Running Examples

```bash
# Run the get-type-offset example
cargo run --bin get-type-offset /sys/kernel/btf/vmlinux 'task_struct' 'pid'
```

## Coding Standards

### Rust Style

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Run `cargo fmt` before committing to ensure consistent formatting
- Run `cargo clippy` and address all warnings

```bash
# Format code
cargo fmt

# Check for common mistakes and style issues
cargo clippy --all-features -- -D warnings
```

### Documentation

- Add doc comments (///) for all public APIs
- Include examples in doc comments where helpful
- Use clear, descriptive names for functions and variables

**Example documentation:**
```rust
/// Returns the byte offset of a field within a structure.
///
/// Given a type ID and a field path (e.g., "field.subfield.member"),
/// this method returns the offset in bytes from the start of the type.
///
/// # Arguments
///
/// * `type_id` - The ID of the type to query
/// * `path` - Dot-separated path to the field
///
/// # Examples
///
/// ```
/// let offset = type_info.offset_of(type_id, "d_name.len")?;
/// println!("Offset: {:?}", offset);
/// ```
///
/// # Errors
///
/// Returns an error if the type ID is invalid or the path cannot be resolved.
pub fn offset_of(&self, type_id: TypeId, path: &str) -> Result<FieldOffset> {
    // implementation
}
```

### Error Handling

- Use `Result` types for operations that can fail
- Provide clear, actionable error messages
- Avoid panicking in library code; use `Result` or `Option` instead

### Performance

- Avoid unnecessary allocations
- Use references where possible
- Consider the performance impact of changes, especially in hot paths
- If adding optional performance features, use feature flags (see the `caching` feature)

## Testing

All contributions must include appropriate tests.

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with all features
cargo test --all-features

# Run a specific test
cargo test test_offset_of
```

### Writing Tests

- Add unit tests in the same file as the code being tested
- Add integration tests in the `tests/` directory for end-to-end scenarios
- Ensure tests are deterministic and don't depend on external state
- Use descriptive test names that explain what is being tested

**Example test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_of_nested_struct() {
        let btf_data = create_test_btf_with_nested_structs();
        let type_info = TypeInformation::new(&btf_data).unwrap();
        let type_id = type_info.id_of("outer_struct").unwrap();

        let offset = type_info.offset_of(type_id, "inner.field").unwrap();

        assert_eq!(offset.byte_offset(), 8);
    }
}
```

## Pull Request Process

1. **Create a branch**: Use a descriptive name (e.g., `fix-parser-panic`, `add-array-support`)

2. **Make your changes**: Follow the coding standards and include tests

3. **Update documentation**: Update README.md or doc comments if needed

4. **Commit your changes**: Write clear, descriptive commit messages
   ```
   Good: "Fix panic when parsing BTF files with DECL_TAG entries"
   Bad: "Fix bug"
   ```

5. **Push to your fork**: `git push origin your-branch-name`

6. **Open a Pull Request**:
   - Provide a clear title and description
   - Reference any related issues (e.g., "Fixes #123")
   - Explain what changed and why
   - Include any testing steps or relevant information

7. **Respond to feedback**: Address review comments and update your PR as needed

8. **Merge**: Once approved, a maintainer will merge your PR

### Pull Request Checklist

Before submitting, ensure:

- [ ] Code follows the project's coding standards
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy` produces no warnings
- [ ] All tests pass (`cargo test`)
- [ ] New tests have been added for new functionality or bug fixes
- [ ] Documentation has been updated if necessary
- [ ] Commit messages are clear and descriptive
- [ ] The PR description explains what changed and why

---

Thank you for contributing to btfparse! Your efforts help make this project better for everyone.
