#[macro_export]
macro_rules! define_common_type_methods {
    ($name:ident) => {
        impl Type for $name {
            /// Returns the type header
            fn header(&self) -> &Header {
                &self.type_header
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
            /// Type header
            type_header: Header,
        }

        define_common_type_methods!($name);

        impl $name {
            /// Creates a new `$name` object
            pub fn new(
                reader: &mut Reader,
                file_header: &FileHeader,
                type_header: Header,
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

                Ok(Self { type_header })
            }
        }
    };

    ($name:ident, $type:ty, $($data_name:ident: $data_type:ty),+) => {
        /// Represents a `$name` type
        #[derive(Debug, Clone)]
        pub struct $name {
            /// Type header
            type_header: Header,

            /// Type data
            data: $type,
        }

        define_common_type_methods!($name);

        impl $name {
            /// Creates a new `$name` object
            pub fn new(
                reader: &mut Reader,
                file_header: &FileHeader,
                type_header: Header,
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
                    type_header,
                    data,
                })
            }

            $(
                /// Returns the `$data_name` field of the type
                pub fn $data_name(&self) -> &$data_type {
                    &self.data.$data_name
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! generate_constructor_dispatcher {
    ($($kind:ident),+) => {
        /// Creates a new `TypeVariant` object based on the given `Header::kind()`
        fn parse_type(kind: Kind, reader: &mut Reader, file_header: &FileHeader, type_header: Header) -> BTFResult<TypeVariant> {
            Ok(match kind {
                $(
                    Kind::$kind => TypeVariant::$kind($kind::new(reader, file_header, type_header)?),
                )+
            })
        }
    };
}
