use crate::btf::parser::{Type, TypeData};
use crate::btf::{Readable, Result as BTFResult};

/// Type information acquired from the BTF data
pub struct TypeInformation {
    type_data: TypeData,
}

impl TypeInformation {
    /// Creates a new `TypeInformation` object
    pub fn new(readable: &dyn Readable) -> BTFResult<Self> {
        Ok(Self {
            type_data: TypeData::new(readable)?,
        })
    }

    /// Returns the specified type by its ID
    pub fn get_type_by_id(&self, type_id: u32) -> Option<&Type> {
        self.type_data.id_to_type.get(&type_id)
    }

    /// Returns the specified type by its name
    pub fn get_type_by_name(&self, name: &str) -> Option<&Type> {
        let type_id = self.type_data.name_to_id.get(name)?;
        self.get_type_by_id(*type_id)
    }
}
