/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::btf::Result as BTFResult;

/// A trait for reading bytes from a source
pub trait Readable {
    /// Reads `buffer.len()` bytes from the given offset
    fn read(&self, offset: u64, buffer: &mut [u8]) -> BTFResult<()>;
}
