/// represents equality operators for searching (e.g. ==, >, and <)
#[derive(Debug, Clone, Copy)]
pub enum EqualityOperator {
    Eq,
    Gt,
    Lt,
}

/// represents different general file size descriptors
#[derive(Clone, Copy, Debug)]
pub enum FileSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
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
/// TODO think on this: 3 different attribute search objects with only the fields each type needs, and each enum variant for [AttributeTypes]
/// TODO gets its corresponding AttributeSearch thing. Makes the code cleaner, prevents a bunch of optional fields, and allows us to do pattern matching
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

#[derive(PartialEq, Eq, Debug)]
enum ParseError {
    /// no matching equality operator found where one is expected
    BadEqualityOperator,
    /// no value for the attribute is passed
    MissingValue,
}

/// determines which equality operator is present in the passed `attr_string`
/// no defaults are assumed. If no equality operator is found or an invalid one is passed, [ParseError::BadEqualityOperator] is returned
fn parse_equality_operator(attr_string: &str) -> Result<EqualityOperator, ParseError> {
    if !attr_string.contains(".") {
        return Err(ParseError::BadEqualityOperator);
    }
    if !attr_string.contains(";") {
        return Err(ParseError::MissingValue);
    }
    // between the . and the ; is our operator
    let period = attr_string.find(".").unwrap();
    let semicolon = attr_string.find(";").unwrap();
    let op = &attr_string[period + 1..semicolon];
    println!("{op}");
    Ok(EqualityOperator::Eq)
}

#[cfg(test)]
mod parse_equality_operator_tests {

    use super::{parse_equality_operator, ParseError};

    #[test]
    fn returns_error_if_no_period() {
        let err = parse_equality_operator("bad_whatevereq;1").unwrap_err();
        assert_eq!(ParseError::BadEqualityOperator, err);
    }

    #[test]
    fn returns_error_if_no_value() {
        assert_eq!(
            ParseError::MissingValue,
            parse_equality_operator("test.eq").unwrap_err()
        );
    }
}
