#[macro_export]
macro_rules! define_common_type_methods {
    ($name:ident) => {
        impl Type for $name {
            /// Returns the type name
            fn name(&self) -> Option<String> {
                self.name.clone()
            }

            /// Returns the raw string section offset
            fn name_offset(&self) -> u32 {
                self.type_header.name_offset()
            }

            /// Returns the `vlen` field of the type header
            fn vlen(&self) -> usize {
                self.type_header.vlen()
            }

            /// Returns the type kind
            fn kind(&self) -> Kind {
                self.type_header.kind()
            }

            /// Returns the `kind_flag` field of the type header
            fn kind_flag(&self) -> bool {
                self.type_header.kind_flag()
            }

            /// Returns the `size_or_type` field of the type header
            fn size_or_type(&self) -> u32 {
                self.type_header.size_or_type()
            }
        }
    };
}

#[macro_export]
macro_rules! define_type {
    ($name:ident) => {
        /// Represents a `$name` type
        #[derive(Debug, Clone)]
        pub struct $name {
            name: Option<String>,
            type_header: TypeHeader,
        }

        define_common_type_methods!($name);

        impl $name {
            /// Creates a new `$name` object
            pub fn new(
                reader: &mut Reader,
                file_header: &FileHeader,
                type_header: TypeHeader,
            ) -> BTFResult<Self> {
                if !matches!(type_header.kind(), Kind::$name) {
                    return Err(BTFError::new(
                        BTFErrorKind::InvalidBTFKind,
                        &format!(
                            "Invalid type kind: {:?} (expected {:?})",
                            type_header.kind(),
                            Kind::$name
                        ),
                    ));
                }

                let name = if type_header.name_offset() != 0 {
                    Some(parse_string(
                        reader,
                        file_header,
                        type_header.name_offset(),
                    )?)
                } else {
                    None
                };

                Ok(Self { name, type_header })
            }
        }
    };

    ($name:ident, $type:ty) => {
        /// Represents a `$name` type
        #[derive(Debug, Clone)]
        pub struct $name {
            name: Option<String>,
            type_header: TypeHeader,
            data: $type,
        }

        define_common_type_methods!($name);

        impl $name {
            /// Creates a new `$name` object
            pub fn new(
                reader: &mut Reader,
                file_header: &FileHeader,
                type_header: TypeHeader,
            ) -> BTFResult<Self> {
                if !matches!(type_header.kind(), Kind::$name) {
                    return Err(BTFError::new(
                        BTFErrorKind::InvalidBTFKind,
                        &format!(
                            "Invalid type kind: {:?} (expected {:?})",
                            type_header.kind(),
                            Kind::$name
                        ),
                    ));
                }

                let name = if type_header.name_offset() != 0 {
                    Some(parse_string(
                        reader,
                        file_header,
                        type_header.name_offset(),
                    )?)
                } else {
                    None
                };

                let required_extra_bytes = <$type>::size(&type_header);
                if required_extra_bytes > 0 {
                    let type_section_start = file_header.hdr_len() + file_header.type_off();
                    let type_section_end = type_section_start + file_header.type_len();

                    if reader.offset() < type_section_start as usize {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypeSectionOffset,
                            "Invalid type section offset",
                        ));
                    }

                    if reader.offset() + required_extra_bytes > type_section_end as usize {
                        return Err(BTFError::new(
                            BTFErrorKind::InvalidTypeSectionOffset,
                            "Invalid type section offset",
                        ));
                    }
                }

                let data = <$type>::new(reader, file_header, &type_header)?;

                Ok(Self {
                    name,
                    type_header,
                    data,
                })
            }

            /// Returns the extra data contained in this type
            pub fn data(&self) -> &$type {
                &self.data
            }
        }
    };
}

#[macro_export]
macro_rules! generate_constructor_dispatcher {
    ($($kind:ident),+) => {
        /// Creates a new `TypeVariant` object based on the given `TypeHeader::kind()`
        fn parse_type(kind: Kind, reader: &mut Reader, file_header: &FileHeader, type_header: TypeHeader) -> BTFResult<TypeVariant> {
            Ok(match kind {
                $(
                    Kind::$kind => TypeVariant::$kind($kind::new(reader, file_header, type_header)?),
                )+
            })
        }
    };
}
