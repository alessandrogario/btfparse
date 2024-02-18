/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

mod reader;
pub use reader::*;

#[cfg(test)]
mod readable_buffer;

#[cfg(test)]
pub use readable_buffer::*;
