use std::{backtrace::Backtrace, collections::HashSet};

use chrono::NaiveDateTime;
use rusqlite::{params, Connection, ToSql};

/// a sql where clause part with named parameter tuple
type WhereClause = (String, Option<(String, String)>);

use crate::model::{
    file_types::FileTypes,
    repository::FileRecord,
    request::attributes::{
        AliasedAttribute, AliasedComparisonTypes, AttributeSearch, AttributeTypes,
        EqualityOperator, FileSizes, FullComparisonAttribute, FullComparisonTypes, NamedAttributes,
        NamedComparisonAttribute,
    },
};

pub fn create_file(file: &FileRecord, con: &Connection) -> Result<u32, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/create_file.sql"))
        .unwrap();

    match pst.insert(params![
        file.name,
        file.size,
        file.create_date,
        file.file_type
    ]) {
        Ok(id) => Ok(id as u32),
        Err(e) => {
            log::error!(
                "Failed to save file record. Nested exception is {e:?}\n{}",
                Backtrace::force_capture()
            );
            Err(e)
        }
    }
}

pub fn get_file(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/get_file_by_id.sql"))
        .unwrap();

    pst.query_row([id], map_file_all_fields)
}

/// returns the full path (excluding root name) of the specified file in the database
pub fn get_file_path(id: u32, con: &Connection) -> Result<String, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!(
            "../assets/queries/file/get_file_path_by_id.sql"
        ))
        .unwrap();
    pst.query_row([id], |row| row.get(0))
}

/// removes the file with the passed id from the database
pub fn delete_file(id: u32, con: &Connection) -> Result<FileRecord, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/file/delete_file_by_id.sql"))
        .unwrap();

    // we need to be able to delete the file off the disk, so we have to return the FileRecord too
    let record = get_file(id, con)?;

    if let Err(e) = pst.execute([id]) {
        log::error!(
            "Failed to delete file by id. Nested exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
        return Err(e);
    }
    Ok(record)
}

/// renames the file with the passed id and links it to the folder with the passed id in the database.
/// This performs no checks, so file name and paths must be checked ahead of time
pub fn update_file(record: &FileRecord, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut update_name_pst =
        con.prepare(include_str!("../assets/queries/file/update_file.sql"))?;
    let mut unlink_file_pst = con.prepare(include_str!(
        "../assets/queries/folder_file/delete_folder_file_by_file_id.sql"
    ))?;
    let FileRecord {
        id: file_id,
        name: file_name,
        parent_id,
        create_date: _,
        size: _,
        file_type,
    } = record;
    // now to rename the file
    update_name_pst.execute(rusqlite::params![file_name, file_type, file_id])?;
    unlink_file_pst.execute([file_id])?;
    // if we specified a parent id, we need to add a link back
    if parent_id.is_some() {
        let mut add_link_pst = con.prepare(include_str!(
            "../assets/queries/folder_file/create_folder_file.sql"
        ))?;
        add_link_pst.execute([file_id, parent_id])?;
    }
    Ok(())
}

/// performs a fuzzy search using the passed criteria.
/// The fuzzy search mashes all the fields together and performs a sql `LIKE` clause on the input
pub fn search_files(criteria: &str, con: &Connection) -> Result<Vec<FileRecord>, rusqlite::Error> {
    let criteria = format!("%{}%", criteria);
    let mut pst = con.prepare(include_str!("../assets/queries/file/search_files.sql"))?;
    let rows = pst.query_map([&criteria], map_file_all_fields)?;
    rows.into_iter().collect()
}

pub fn search_files_by_tags(
    tags: &HashSet<String>,
    con: &Connection,
) -> Result<HashSet<FileRecord>, rusqlite::Error> {
    let base_sql_string = include_str!("../assets/queries/file/get_files_by_all_tags.sql");
    // need to fill out the in clause and the count clause
    let joined_tags = tags
        .iter()
        .map(|t| format!("'{}'", t.replace('\'', "''")))
        .reduce(|combined, current| format!("{combined},{current}"))
        .unwrap();
    let replaced_string = base_sql_string
        .to_string()
        .replace("?1", joined_tags.as_str())
        .replace("?2", tags.len().to_string().as_str());
    let mut pst = con.prepare(replaced_string.as_str())?;
    let res = pst.query_map([], map_file_all_fields)?;
    res.into_iter().collect()
}

pub fn search_files_by_attributes(
    attributes: AttributeSearch,
    con: &Connection,
) -> Result<HashSet<FileRecord>, rusqlite::Error> {
    let (where_clause, params) = build_search_attribute_sql(attributes);
    let params: Vec<(&str, &dyn ToSql)> = params
        .iter()
        .map(|(pname, pvalue)| (pname.as_str(), pvalue as &dyn ToSql))
        .collect();
    let mut pst = con.prepare(&where_clause)?;
    let res = pst.query_map(&params[..], map_file_all_fields)?;
    res.into_iter().collect()
}

pub fn create_file_preview(
    file_id: u32,
    contents: Vec<u8>,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/file/create_file_preview.sql"
    ))?;
    pst.insert(params![file_id, contents])?;
    Ok(())
}

/// retrieves all [FileRecord]s from the database
pub fn get_all_files(con: &Connection) -> Result<Vec<FileRecord>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/file/get_all_files.sql"))?;
    let rows = pst.query_map([], map_file_all_fields)?;
    rows.into_iter().collect()
}

pub fn get_file_preview(file_id: u32, con: &Connection) -> Result<Vec<u8>, rusqlite::Error> {
    let mut pst = con.prepare(&format!(
        include_str!("../assets/queries/file/get_file_preview.sql"),
        file_id
    ))?;
    let res: Vec<u8> = pst.query_row([], |row| row.get(0))?;
    Ok(res)
}

pub fn delete_file_preview(file_id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/file/delete_file_preview.sql"
    ))?;
    pst.execute(params![file_id])?;
    Ok(())
}

pub fn get_all_file_ids(con: &Connection) -> Result<Vec<u32>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/file/get_all_file_ids.sql"))?;
    let res = pst.query_map([], |row| row.get(0))?;
    res.into_iter().collect()
}

pub fn map_file_all_fields(row: &rusqlite::Row) -> Result<FileRecord, rusqlite::Error> {
    let id = row.get(0)?;
    let name = row.get(1)?;
    // not that I ever think this will be used for files this large - sqlite3 can store up to 8 bytes for a numeric value with a sign, so i64 it is
    let size: i64 = row.get(2)?;
    let create_date: NaiveDateTime = row.get(3)?;
    let file_type: String = row.get(4)?;
    let file_type: FileTypes = FileTypes::from(&file_type as &str);
    let parent_id = row.get(5)?;
    Ok(FileRecord {
        id,
        name,
        parent_id,
        create_date,
        size: size.try_into().unwrap_or(0),
        file_type,
    })
}

/// builds the entire sql query to search for files by attributes
///
/// The first part of the returned tuple is the sql query with parameter placeholders.
/// The second part of the returned tuple is the collection of parameters to be used for those placeholders
fn build_search_attribute_sql(attributes: AttributeSearch) -> (String, Vec<(String, String)>) {
    let base_sql = r"select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    f.type,
    ff.folderId
from
    FileRecords f
    left join folder_files ff on ff.fileId = f.id
where ";
    let mut where_clause = base_sql.to_string();
    let mut params: Vec<(String, String)> = Vec::with_capacity(attributes.len());
    for (arg_count, attr) in attributes.clone().into_iter().enumerate() {
        let (sql, param_data) = convert_attribute_to_where_clause(attr, arg_count);
        if let Some(p_data) = param_data {
            params.push(p_data);
        }
        where_clause += sql.as_str();
        if arg_count < attributes.len() - 1 {
            where_clause += " AND ";
        }
    }
    (where_clause, params)
}

/// converts the passed attr to a tuple with both the where clause part and the parameters needed to populate
/// that where clause in a parameterized sql query
/// * `attr` the attribute to generate parameters for
/// * `counter` the counter used to keep track of how many parameters there are. This is _only_ used to make sure parameter names are unique, and is not updated by this function
fn convert_attribute_to_where_clause(attr: AttributeTypes, counter: usize) -> WhereClause {
    match attr {
        AttributeTypes::FullComp(at) => convert_full_comp_attribute_to_where_clause(at, counter),
        AttributeTypes::Named(at) => convert_named_comp_attribute_to_where_clause(at, counter),
        AttributeTypes::Aliased(at) => convert_aliased_attribute_to_where_clause(at, counter),
    }
}

/// converts the passed `attr` to a string that can be used in a sql where clause and the parameters needed to populate
/// that where clause in a parameterized sql query
/// * `attr` the attribute to generate parameters for
/// * `counter` the counter used to keep track of how many parameters there are. This is _only_ used to make sure parameter names are unique, and is not updated by this function
fn convert_full_comp_attribute_to_where_clause(
    attr: FullComparisonAttribute,
    counter: usize,
) -> WhereClause {
    let field_name = match attr.field {
        FullComparisonTypes::FileSize => "fileSize",
        FullComparisonTypes::DateCreated => "dateCreated",
    };
    let field_placeholder = format!(":{field_name}{counter}");
    let op: &str = attr.operator.into();
    let sql = format!("{field_name} {op} {field_placeholder}");
    (sql, Some((field_placeholder, attr.value)))
}

/// converts the passed `attr` to a string that can be used in a sql where clause and the parameters needed to populate
/// that where clause in a parameterized sql query
/// * `attr` the attribute to generate parameters for
/// * `counter` the counter used to keep track of how many parameters there are. This is _only_ used to make sure parameter names are unique, and is not updated by this function
fn convert_named_comp_attribute_to_where_clause(
    attr: NamedComparisonAttribute,
    counter: usize,
) -> WhereClause {
    let field_name = match attr.field {
        NamedAttributes::FileType => "type",
    };
    let op: &str = attr.operator.into();
    let field_placeholder = format!(":{field_name}{counter}");
    let sql = format!("{field_name} {op} {field_placeholder}");
    (sql, Some((field_placeholder, attr.value)))
}

/// converts the passed `attr` to a string that can be used in a sql where clause and the parameters needed to populate
/// that where clause in a parameterized sql query
/// * `attr` the attribute to generate parameters for
/// * `counter` the counter used to keep track of how many parameters there are. This is _only_ used to make sure parameter names are unique, and is not updated by this function
fn convert_aliased_attribute_to_where_clause(
    attr: AliasedAttribute,
    counter: usize,
) -> WhereClause {
    if attr.field == AliasedComparisonTypes::FileSize {
        return convert_aliased_file_size_to_where_clause(attr);
    }
    let field_name = match attr.field {
        AliasedComparisonTypes::FileSize => "fileSize",
    };
    let field_placeholder = format!(":{field_name}{counter}");
    let sql = format!("{field_name} = {field_placeholder}");
    (sql, Some((field_placeholder, attr.value)))
}

fn convert_aliased_file_size_to_where_clause(attr: AliasedAttribute) -> WhereClause {
    // at this point the attr value should be validated, but just in case we need a default
    let size = FileSizes::try_from(&attr.value).unwrap_or(FileSizes::ExtraLarge);
    // lt - less than min bound (except for tiny in which case it gets clamped down to the max tiny value)
    // gt - greater than max bound (except for ExtraLarge in which case it gets clamped down to the min extra large value)
    // eq - min <= value < max
    // neq - value < min || value >= max (probably not gonna be used much, but still need to fulfill all ops)
    let (min, max) = determine_file_size_range(size);
    let sql = match attr.operator {
        EqualityOperator::Lt => {
            // nothing can be less than tiny, so clamp it up to ask for tiny
            if size == FileSizes::Tiny {
                format!("fileSize <= {max}")
            } else {
                format!("fileSize < {min}")
            }
        }
        EqualityOperator::Gt => {
            // nothing can be greater than extra large, so clamp it down to ask for extra large
            if size == FileSizes::ExtraLarge {
                format!("fileSize >= {min}")
            } else {
                format!("fileSize >= {max}")
            }
        }
        EqualityOperator::Eq => format!("(fileSize >= {min} AND fileSize < {max})"),
        EqualityOperator::Neq => format!("(fileSize < {min} OR fileSize >= {max})"),
    };
    (sql, None)
}

/// determines a range of positive integers for a give [FileSizes] alias. These ranges are as follows:
/// - [FileSizes::Tiny]: 0B - 500KiB
/// - [FileSizes::Small]: 500KiB - 10MiB
/// - [FileSizes::Medium]: 10MiB - 100MiB
/// - [FileSizes::Large]: 100MiB - 1GiB
/// - [FileSizes::ExtraLarge]: 1GiB - u64 max
///
/// u64 is used because that is the sqlite max integer value (https://www.sqlite.org/datatype3.html#storage_classes_and_datatypes)
fn determine_file_size_range(size: FileSizes) -> (u64, u64) {
    static KIB: u64 = 1024;
    static MIB: u64 = 1024 * 1024;
    static GIB: u64 = 1024 * 1024 * 1024;
    match size {
        FileSizes::Tiny => (0, 500 * KIB),
        FileSizes::Small => (500 * KIB, 10 * MIB),
        FileSizes::Medium => (10 * MIB, 100 * MIB),
        FileSizes::Large => (100 * MIB, GIB),
        FileSizes::ExtraLarge => (GIB, u64::MAX),
    }
}

#[cfg(test)]
mod get_files_by_all_tags_tests {
    use std::collections::HashSet;

    use rusqlite::Connection;

    use crate::model::file_types::FileTypes;
    use crate::model::repository::FileRecord;
    use crate::repository::file_repository::search_files_by_tags;
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_file_db_entry, create_tag_files, now, refresh_db};

    #[test]
    fn returns_files_with_all_tags() {
        refresh_db();
        let con: Connection = open_connection();
        create_file_db_entry("bad", None);
        create_file_db_entry("has some", None); // 2
        create_file_db_entry("has all", None); // 3
        create_file_db_entry("also has all", None); // 4
                                                    // add tags
        create_tag_files("tag1", vec![2, 3, 4]);
        create_tag_files("asdf", vec![3, 4]);
        create_tag_files("fda", vec![2, 3, 4]);

        let res = search_files_by_tags(
            &HashSet::from(["tag1".to_string(), "fda".to_string(), "asdf".to_string()]),
            &con,
        )
        .unwrap()
        .into_iter()
        .collect::<Vec<FileRecord>>();
        con.close().unwrap();
        assert_eq!(2, res.len());
        assert!(res.contains(&FileRecord {
            id: Some(3),
            name: "has all".to_string(),
            parent_id: None,
            create_date: now(),
            size: 0,
            file_type: FileTypes::Unknown
        }));
        assert!(res.contains(&FileRecord {
            id: Some(4),
            name: "also has all".to_string(),
            parent_id: None,
            create_date: now(),
            size: 0,
            file_type: FileTypes::Unknown
        }));
        cleanup();
    }
}

#[cfg(test)]
mod file_preview_tests {
    use rusqlite::Connection;

    use crate::repository::file_repository::{
        create_file_preview, delete_file_preview, get_file_preview,
    };
    use crate::repository::open_connection;
    use crate::test::{cleanup, create_file_db_entry, refresh_db};

    #[test]
    fn test_create_file_preview_works() {
        refresh_db();
        let con: Connection = open_connection();
        create_file_db_entry("test.txt", None);
        let preview_contents: Vec<u8> = vec![72, 105];

        create_file_preview(1, preview_contents.clone(), &con).unwrap();
        let preview = get_file_preview(1, &con).unwrap();

        assert_eq!(preview_contents, preview);

        cleanup();
    }

    #[test]
    fn test_delete_file_preview_works() {
        refresh_db();
        let con: Connection = open_connection();
        create_file_db_entry("test.txt", None);
        create_file_preview(1, vec![72, 105], &con).unwrap();

        delete_file_preview(1, &con).unwrap();

        let err = get_file_preview(1, &con).unwrap_err();
        assert_eq!(rusqlite::Error::QueryReturnedNoRows, err);

        cleanup();
    }
}

#[cfg(test)]
mod create_file_tests {
    use crate::{
        model::{file_types::FileTypes, repository::FileRecord},
        repository::open_connection,
        test::{cleanup, create_folder_db_entry, now, refresh_db},
    };

    #[test]
    fn saves_all_fields_to_db() {
        refresh_db();
        create_folder_db_entry("whatever", None);
        let create_date = now();
        let name = "bg3_bugbear.mp4".to_string();
        let size = 525600;
        let file_type = FileTypes::Video;
        let record = FileRecord {
            id: None,
            name: name.clone(),
            // this creates the file but doesn't handle linking it to the folder
            parent_id: None,
            create_date,
            size,
            file_type,
        };
        let con = open_connection();
        super::create_file(&record, &con).unwrap();
        let retrieved = super::get_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(1, retrieved.id.unwrap());
        assert_eq!(name, retrieved.name);
        assert_eq!(create_date, retrieved.create_date);
        assert_eq!(size, retrieved.size);
        assert_eq!(file_type, retrieved.file_type);
        cleanup();
    }
}

#[cfg(test)]
mod convert_named_comp_attribute_to_where_clause {
    use crate::model::request::attributes::EqualityOperator;

    use super::*;

    #[test]
    fn maps_field_name_properly() {
        let attr = NamedComparisonAttribute {
            field: NamedAttributes::FileType,
            value: "test".to_string(),
            operator: EqualityOperator::Eq,
        };
        let (sql, _var) = convert_named_comp_attribute_to_where_clause(attr, 23);
        assert!(sql.starts_with("type"));
    }

    #[test]
    fn uses_proper_variable_counter() {
        let attr = NamedComparisonAttribute {
            field: NamedAttributes::FileType,
            value: "test".to_string(),
            operator: EqualityOperator::Eq,
        };
        let (sql, _var) = convert_named_comp_attribute_to_where_clause(attr, 24);
        assert!(sql.ends_with(":type24"));
    }

    #[test]
    fn returns_proper_variable_value() {
        let attr = NamedComparisonAttribute {
            field: NamedAttributes::FileType,
            value: "test".to_string(),
            operator: EqualityOperator::Eq,
        };
        let (_, params) = convert_named_comp_attribute_to_where_clause(attr, 23);
        let (_, var) = params.unwrap();
        assert_eq!("test".to_string(), var);
    }

    #[test]
    fn builds_full_clause() {
        let attr = NamedComparisonAttribute {
            field: NamedAttributes::FileType,
            value: "test".to_string(),
            operator: EqualityOperator::Neq,
        };
        let (sql, _) = convert_named_comp_attribute_to_where_clause(attr, 23);
        assert_eq!("type <> :type23".to_string(), sql);
    }

    #[test]
    fn uses_correct_parameter_name() {
        let attr = NamedComparisonAttribute {
            field: NamedAttributes::FileType,
            value: "test".to_string(),
            operator: EqualityOperator::Eq,
        };
        let (_, params) = convert_named_comp_attribute_to_where_clause(attr, 10);
        let (var_name, _) = params.unwrap();
        assert_eq!(var_name, ":type10".to_string());
    }
}

#[cfg(test)]
mod convert_full_comp_attribute_to_where_clause {
    use super::*;
    use crate::model::request::attributes::*;

    #[test]
    fn maps_field_name_properly() {
        let attr = FullComparisonAttribute {
            field: FullComparisonTypes::FileSize,
            value: "test".to_string(),
            operator: EqualityOperator::Eq,
        };
        let (sql, _) = convert_full_comp_attribute_to_where_clause(attr, 23);
        assert!(sql.starts_with("fileSize"));
    }

    #[test]
    fn uses_proper_variable_counter() {
        let attr = FullComparisonAttribute {
            field: FullComparisonTypes::DateCreated,
            value: "test".to_string(),
            operator: EqualityOperator::Gt,
        };
        let (sql, _) = convert_full_comp_attribute_to_where_clause(attr, 38);
        assert!(sql.ends_with(":dateCreated38"));
    }

    #[test]
    fn returns_proper_variable_value() {
        let attr = FullComparisonAttribute {
            field: FullComparisonTypes::FileSize,
            value: "test".to_string(),
            operator: EqualityOperator::Eq,
        };
        let (_, params) = convert_full_comp_attribute_to_where_clause(attr, 23);
        let (_, var) = params.unwrap();
        assert_eq!("test".to_string(), var);
    }

    #[test]
    fn builds_full_clause() {
        let attr = FullComparisonAttribute {
            field: FullComparisonTypes::DateCreated,
            value: "test".to_string(),
            operator: EqualityOperator::Neq,
        };
        let (sql, _) = convert_full_comp_attribute_to_where_clause(attr, 23);
        assert_eq!("dateCreated <> :dateCreated23".to_string(), sql);
    }
}

#[cfg(test)]
mod build_search_attribute_sql {
    use super::*;
    use crate::model::request::attributes::*;

    #[test]
    fn handles_single_param() {
        let attributes = vec![AttributeTypes::FullComp(FullComparisonAttribute {
            field: FullComparisonTypes::FileSize,
            operator: EqualityOperator::Eq,
            value: "5000".to_string(),
        })];
        let expected = r"select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    f.type,
    ff.folderId
from
    FileRecords f
    left join folder_files ff on ff.fileId = f.id
where fileSize = :fileSize0";
        let (actual, _) = build_search_attribute_sql(AttributeSearch { attributes });
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_multiple_params() {
        let attributes = vec![
            AttributeTypes::Aliased(AliasedAttribute {
                field: AliasedComparisonTypes::FileSize,
                value: FileSizes::Large.to_string(),
                operator: EqualityOperator::Eq,
            }),
            AttributeTypes::Named(NamedComparisonAttribute {
                field: NamedAttributes::FileType,
                value: FileTypes::Image.to_string(),
                operator: EqualityOperator::Neq,
            }),
        ];
        let expected = r"select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    f.type,
    ff.folderId
from
    FileRecords f
    left join folder_files ff on ff.fileId = f.id
where (fileSize >= 104857600 AND fileSize < 1073741824) AND type <> :type1";
        let (actual, _) = build_search_attribute_sql(AttributeSearch { attributes });
        assert_eq!(expected, actual);
    }
}

#[cfg(test)]
mod search_files_by_attributes {
    use chrono::NaiveTime;

    use super::*;
    use crate::{
        model::request::attributes::*,
        repository::open_connection,
        test::{cleanup, now, refresh_db},
    };

    #[test]
    fn properly_retrieves_files_with_1_attr() {
        refresh_db();
        let good = FileRecord {
            id: None,
            name: "good.txt".to_string(),
            parent_id: None,
            create_date: now(),
            size: 5 * 1024 * 1024 * 1024,
            file_type: FileTypes::Text,
        }
        .save_to_db();
        let bad = FileRecord {
            id: None,
            name: "bad.gif".to_string(),
            parent_id: None,
            create_date: now(),
            size: 5 * 1024,
            file_type: FileTypes::Image,
        }
        .save_to_db();
        let attributes = vec![AttributeTypes::FullComp(FullComparisonAttribute {
            field: FullComparisonTypes::FileSize,
            operator: EqualityOperator::Gt,
            value: "1073741824".to_string(),
        })];
        let con = open_connection();
        let res = search_files_by_attributes(AttributeSearch { attributes }, &con);
        con.close().unwrap();
        let expected: HashSet<FileRecord> = [good].into_iter().collect();
        assert_eq!(Ok(expected), res);
        cleanup();
    }

    #[test]
    fn properly_retrieves_files_with_multiple_attr() {
        let date = chrono::NaiveDate::from_ymd_opt(2020, 01, 15).unwrap();
        let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let date_time = NaiveDateTime::new(date, time);
        refresh_db();
        let expected: HashSet<FileRecord> = [
            FileRecord {
                id: None,
                name: "good".to_string(),
                parent_id: None,
                create_date: date_time,
                // `Large` size
                size: 100 * 1024 * 1024,
                file_type: FileTypes::Image,
            }
            .save_to_db(),
            FileRecord {
                id: None,
                name: "good2".to_string(),
                parent_id: None,
                create_date: date_time,
                // `ExtraLarge` size
                size: 1024 * 1024 * 1024,
                file_type: FileTypes::Text,
            }
            .save_to_db(),
        ]
        .into_iter()
        .collect();
        FileRecord {
            id: None,
            name: "bad".to_string(),
            parent_id: None,
            create_date: date_time.checked_add_days(chrono::Days::new(10)).unwrap(),
            // `Medium` size
            size: 10 * 1024 * 1024,
            file_type: FileTypes::Application,
        }
        .save_to_db();
        FileRecord {
            id: None,
            name: "bad2".to_string(),
            parent_id: None,
            create_date: date_time,
            // `small` size
            size: 10 * 1024,
            file_type: FileTypes::Image,
        }
        .save_to_db();
        // must not be an application, must be newer than 5 days ago, and must be larger than medium
        let search = AttributeSearch {
            attributes: vec![
                AttributeTypes::Named(NamedComparisonAttribute {
                    field: NamedAttributes::FileType,
                    value: FileTypes::Application.to_string(),
                    operator: EqualityOperator::Neq,
                }),
                AttributeTypes::FullComp(FullComparisonAttribute {
                    field: FullComparisonTypes::DateCreated,
                    operator: EqualityOperator::Gt,
                    value: "2020-01-10 00:00:00".to_string(),
                }),
                AttributeTypes::Aliased(AliasedAttribute {
                    field: AliasedComparisonTypes::FileSize,
                    value: FileSizes::Medium.to_string(),
                    operator: EqualityOperator::Gt,
                }),
            ],
        };
        let con = open_connection();
        let actual = search_files_by_attributes(search, &con);
        con.close().unwrap();
        assert_eq!(Ok(expected), actual);
        cleanup();
    }
}

#[cfg(test)]
mod convert_aliased_file_size_to_where_clause {
    use super::*;
    use crate::model::request::attributes::*;

    #[test]
    fn handles_lt() {
        let attr = AliasedAttribute {
            field: AliasedComparisonTypes::FileSize,
            value: FileSizes::Medium.to_string(),
            operator: EqualityOperator::Lt,
        };
        let expected = "fileSize < 10485760";
        let (actual, _) = convert_aliased_file_size_to_where_clause(attr);
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_lt_for_tiny() {
        let attr = AliasedAttribute {
            field: AliasedComparisonTypes::FileSize,
            value: FileSizes::Tiny.to_string(),
            operator: EqualityOperator::Lt,
        };
        let expected = "fileSize <= 512000";
        let (actual, _) = convert_aliased_file_size_to_where_clause(attr);
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_gt() {
        let attr = AliasedAttribute {
            field: AliasedComparisonTypes::FileSize,
            value: FileSizes::Medium.to_string(),
            operator: EqualityOperator::Gt,
        };
        let expected = "fileSize >= 104857600";
        let (actual, _) = convert_aliased_file_size_to_where_clause(attr);
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_gt_for_extra_large() {
        let attr = AliasedAttribute {
            field: AliasedComparisonTypes::FileSize,
            value: FileSizes::ExtraLarge.to_string(),
            operator: EqualityOperator::Gt,
        };
        let expected = "fileSize >= 1073741824";
        let (actual, _) = convert_aliased_file_size_to_where_clause(attr);
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_eq_for_range() {
        let attr = AliasedAttribute {
            field: AliasedComparisonTypes::FileSize,
            value: FileSizes::Medium.to_string(),
            operator: EqualityOperator::Eq,
        };
        let expected = "(fileSize >= 10485760 AND fileSize < 104857600)";
        let (actual, _) = convert_aliased_file_size_to_where_clause(attr);
        assert_eq!(expected, actual);
    }

    #[test]
    fn handles_neq_for_outside_range() {
        let attr = AliasedAttribute {
            field: AliasedComparisonTypes::FileSize,
            value: FileSizes::Medium.to_string(),
            operator: EqualityOperator::Neq,
        };
        let expected = "(fileSize < 10485760 OR fileSize >= 104857600)";
        let (actual, _) = convert_aliased_file_size_to_where_clause(attr);
        assert_eq!(expected, actual);
    }
}
