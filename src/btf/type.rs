/*
  Copyright (c) 2024-present, Alessandro Gario
  All rights reserved.

  This source code is licensed in accordance with the terms specified in
  the LICENSE file found in the root directory of this source tree.
*/

use crate::btf::Header;

/// Common methods for all BTF types
pub trait Type {
    /// Returns the type header
    fn header(&self) -> &Header;
}
