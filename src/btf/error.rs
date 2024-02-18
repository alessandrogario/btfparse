/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use std::{io, result::Result as StandardResult};

/// Error kinds used by the `reader` module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// An IO error has occurred
    IOError,

    /// The end of the file has been reached
    EOF,

    /// The offset is invalid, as there are not enough bytes left to complete the read request
    InvalidOffset,

    /// The BTF header magic number is invalid
    InvalidMagic,

    /// Invalid BTF kind specified in the type info flags
    InvalidBTFKind,

    /// Invalid string offset
    InvalidStringOffset,

    /// The string is not correctly null terminated
    InvalidString,

    /// Unsupported BTF type encountered
    UnsupportedType,

    /// Invalid type header attribute
    InvalidTypeHeaderAttribute,

    /// Invalid type section offset
    InvalidTypeSectionOffset,

    /// The given type path is invalid
    InvalidTypePath,

    /// Invalid type id
    InvalidTypeID,

    /// The specified BTF type id is not sized
    NotSized,

    /// Found a bitfield in the middle of a type path resolution
    UnexpectedBitfield,
}

/// An error type for the `reader` module
#[derive(Debug)]
pub struct Error {
    /// The error kind
    kind: ErrorKind,

    /// The error message
    message: String,
}

/// A `Result` type for the `btf` module
pub type Result<T> = StandardResult<T, Error>;

impl Error {
    /// Creates a new `Error` instance
    pub fn new(kind: ErrorKind, message: &str) -> Error {
        Error {
            kind,
            message: message.to_owned(),
        }
    }

    /// Returns the error kind
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<io::Error> for Error {
    /// Converts an `io::Error` into a reader error
    fn from(error: io::Error) -> Self {
        Error {
            kind: ErrorKind::IOError,
            message: error.to_string(),
        }
    }
}
