use std::str::FromStr;

/// represents equality operators for searching (e.g. ==, >, and <)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqualityOperator {
    Eq,
    Gt,
    Lt,
}

impl TryFrom<&str> for EqualityOperator {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.to_ascii_lowercase();
        match value.as_str() {
            "eq" => Ok(Self::Eq),
            "lt" => Ok(Self::Lt),
            "gt" => Ok(Self::Gt),
            _ => Err(ParseError::BadEqualityOperator(format!(
                "{value} is not a valid equality operator. Valid ops are `eq`, `lt`, and `gt`"
            ))),
        }
    }
}

/// represents different general file size descriptors
#[derive(Clone, Copy, Debug)]
pub enum FileSizes {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

impl TryFrom<&str> for FileSizes {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.to_ascii_lowercase();
        match value.as_str() {
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            "extralarge" => Ok(Self::ExtraLarge),
            default => Err(ParseError::BadValue(format!(
                "{default} is not a valid file size name"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AttributeTypes {
    /// full comparison attribute
    FullComp(FullComparisonAttribute),
    /// attributes whose values can be one of a specific name
    Named(NamedComparisonAttribute),
    /// attributes with values that are aliased to a specific name (e.g. 1Gb being [FileSize::ExtraLarge])
    Aliased(AliasedAttribute),
}

#[derive(Debug, Clone, Copy)]
pub enum FullComparisonTypes {
    FileSize,
    DateCreated,
}

/// used to force compile-time handling of all aliased attributes. Not useful right now, but if we ever get another field to search on
/// we can guarantee it's covered
#[derive(Debug, Clone, Copy)]
pub enum AliasedComparisonTypes {
    FileSize,
}

#[derive(Debug, Clone)]
pub struct FullComparisonAttribute {
    pub comparison_type: FullComparisonTypes,
    pub operator: EqualityOperator,
    pub value: String,
}

/// used to force compile-time handling of all named attributes. Not useful right now, but if we ever get another field to search on
/// we can guarantee it's covered
#[derive(Debug, Clone, Copy)]
pub enum NamedAttributes {
    FileType,
}

#[derive(Debug, Clone)]
pub struct NamedComparisonAttribute {
    pub field: NamedAttributes,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct AliasedAttribute {
    pub field: AliasedComparisonTypes,
    pub value: String,
}

/// represents an attribute search feature.
///
/// There are multiple attribute search types.
/// - size and date are `full comparison attributes`, where we can use every
/// instance of the [EqualityOperator] to search on them
/// - file type is a `named attributed`, where the list of allowed search values are determined by a specific list.
/// - size can also be an `aliased attribute`, where specific values have titles (see [FileSize])
pub struct AttributeSearch {
    pub attributes: Vec<AttributeTypes>,
}

impl From<Vec<String>> for AttributeSearch {
    ///
    /// format for param:
    /// - full comparison: `<field>.<op>;<value>`
    /// - named attribute: `<field>;<value>`
    /// - aliased attribute: `<field>;<value>`
    fn from(value: Vec<String>) -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub enum ParseError {
    /// no matching equality operator found where one is expected
    BadEqualityOperator(String),
    /// no value for the attribute is passed
    MissingValue(String),
    /// (for named values) a bad value was passed
    BadValue(String),
}

// we don't care about the error message when dealing with equality for error messages
impl PartialEq<ParseError> for ParseError {
    fn eq(&self, other: &ParseError) -> bool {
        match (self, other) {
            (Self::BadEqualityOperator(_), Self::BadEqualityOperator(_)) => true,
            (Self::MissingValue(_), Self::MissingValue(_)) => true,
            _ => false,
        }
    }
}

/// determines which equality operator is present in the passed `attr_string`
/// no defaults are assumed. If no equality operator is found or an invalid one is passed, [ParseError::BadEqualityOperator] is returned
fn parse_operator(attr_string: &str) -> Result<EqualityOperator, ParseError> {
    validate_format(attr_string)?;
    // between the . and the ; is our operator
    let period = attr_string.find('.').unwrap();
    let semicolon = attr_string.find(';').unwrap();
    let op = &attr_string[period + 1..semicolon];
    op.try_into()
}

fn validate_format(attr_string: &str) -> Result<(), ParseError> {
    return if !attr_string.contains('.') {
        Err(ParseError::BadEqualityOperator(format!(
            "invalid attribute search {attr_string}: must contain . to separate field from op"
        )))
    } else if !attr_string.contains(';') {
        Err(ParseError::MissingValue(format!(
            "invalid attribute search {attr_string}: must contain a ; to separate op from value"
        )))
    } else {
        Ok(())
    };
}

/// parses and validates the passed `attr_string` into a valid [AttributeTypes] instance
///
/// - fileSize and dateCreated are [FullComparisonAttribute]s, where we can use every instance of the [EqualityOperator] to search on them
/// - fileType is a [NamedComparisonAttribute]s, where the list of allowed search values are determined by a specific list.
/// - fileSize can also be an [AliasedAttribute]s, where specific values have titles (see [FileSize])
fn parse_attribute(attr_string: &str) -> Result<AttributeTypes, ParseError> {
    validate_format(attr_string)?;
    let field_name = parse_field(attr_string);
    let op = parse_operator(attr_string)?;
    let value = parse_value(attr_string);
    // Since size can be shared between 2 different search types, we might have to do some stupid/ugly stuff. I want a clean way though...
    todo!()
}

/// parses an attribute search for either a [FullComparisonAttribute] or an [AliasedAttribute]
fn parse_file_size(operator: EqualityOperator, value: &str) -> Result<AttributeTypes, ParseError> {
    return if FileSizes::try_from(value).is_ok() {
        if operator == EqualityOperator::Eq {
            Ok(AttributeTypes::Aliased(AliasedAttribute {
                field: AliasedComparisonTypes::FileSize,
                value: value.to_string(),
            }))
        } else {
            Err(ParseError::BadEqualityOperator(format!(
                "{operator:?} is not a valid operator when comparing fileSize to an alias"
            )))
        }
    } else if usize::from_str(value).is_ok() {
        Ok(AttributeTypes::FullComp(FullComparisonAttribute {
            comparison_type: FullComparisonTypes::FileSize,
            operator,
            value: value.to_string(),
        }))
    } else {
        Err(ParseError::BadValue(format!(
            "{value} is not a valid byte size for files"
        )))
    };
}

/// returns the field name part of the passed `attr_string`.
/// This does not do any validation, and assumes that the str has been validated beforehand
fn parse_field<'a>(attr_string: &'a str) -> &'a str {
    let period = attr_string.find('.').unwrap();
    &attr_string[0..period]
}

/// returns the value part of the passed `attr_string`.
/// This does not do any validation, and assumes that the str ahs been validated beforehand
fn parse_value<'a>(attr_string: &'a str) -> &'a str {
    let semicolon = attr_string.find(';').unwrap();
    &attr_string[semicolon + 1..]
}

#[cfg(test)]
mod validate_format_tests {

    use super::{parse_operator, ParseError};

    #[test]
    fn returns_error_if_no_period() {
        let err = parse_operator("bad_whatevereq;1").unwrap_err();
        assert_eq!(ParseError::BadEqualityOperator(String::new()), err);
    }

    #[test]
    fn returns_error_if_no_value() {
        assert_eq!(
            ParseError::MissingValue(String::new()),
            parse_operator("test.eq").unwrap_err()
        );
    }
}

#[cfg(test)]
mod parse_operator_tests {
    use super::*;

    #[test]
    fn works_for_valid_ops() {
        assert_eq!(EqualityOperator::Eq, parse_operator("test.eq;5").unwrap());
        assert_eq!(EqualityOperator::Lt, parse_operator("test.lt;5").unwrap());
        assert_eq!(EqualityOperator::Gt, parse_operator("test.gt;5").unwrap());
    }
}

#[cfg(test)]
mod parse_field_tests {
    use super::*;

    #[test]
    fn pulls_out_right_part() {
        let attr = "whatever.op;value";
        assert_eq!("whatever", parse_field(attr));
    }
}

#[cfg(test)]
mod parse_value_tests {
    use super::*;

    #[test]
    fn pulls_out_right_part() {
        let attr = "whatever.op;value";
        assert_eq!("value", parse_value(attr));
    }
}

#[cfg(test)]
mod parse_file_size_tests {
    use super::*;

    #[test]
    fn requires_op_to_be_eq() {
        use crate::test::fail;
        fail();
    }

    #[test]
    fn succesfully_returns_aliased_if_size_name_is_passed() {
        use crate::test::fail;
        fail();
    }

    #[test]
    fn successfully_returns_full_comp_if_no_name_is_passed() {
        use crate::test::fail;
        fail();
    }

    #[test]
    fn full_comp_requires_numeric_byte_value() {
        use crate::test::fail;
        fail();
    }
}
