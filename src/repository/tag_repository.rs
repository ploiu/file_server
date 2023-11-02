use rusqlite::Connection;

use crate::model::repository;

/// creates a new tag in the database. This does not check if the tag already exists,
/// so the caller must check that themselves
pub fn create_tag(title: String, con: &Connection) -> Result<repository::Tag, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/tags/create_tag.sql"))
        .unwrap();
    let id = pst.insert(rusqlite::params![title])? as u32;
    Ok(repository::Tag { id, title })
}

/// searches for a tag that case-insensitively matches that passed title.
///
/// if `None` is returned, that means there was no match
pub fn get_tag_by_title(
    title: String,
    con: &Connection,
) -> Result<Option<repository::Tag>, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/tags/get_by_title.sql"))
        .unwrap();
    return match pst.query_row(rusqlite::params![title], tag_mapper) {
        Ok(tag) => Ok(Some(tag)),
        Err(e) => {
            // no tag found
            return if e == rusqlite::Error::QueryReturnedNoRows {
                Ok(None)
            } else {
                eprintln!("Failed to get tag by name, error is {:?}", e);
                Err(e)
            };
        }
    };
}

pub fn get_tag(id: u32, con: &Connection) -> Result<repository::Tag, rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/tags/get_by_id.sql"))
        .unwrap();
    return Ok(pst.query_row(rusqlite::params![id], tag_mapper)?);
}

/// updates the past tag. Checking to make sure the tag exists needs to be done on the caller's end
pub fn update_tag(tag: repository::Tag, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/tags/update_tag.sql"))
        .unwrap();
    pst.execute(rusqlite::params![tag.title, tag.id])?;
    Ok(())
}

pub fn delete_tag(id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con
        .prepare(include_str!("../assets/queries/tags/delete_tag.sql"))
        .unwrap();
    pst.execute(rusqlite::params![id])?;
    Ok(())
}

fn tag_mapper(row: &rusqlite::Row) -> Result<repository::Tag, rusqlite::Error> {
    let id: u32 = row.get(0)?;
    let title: String = row.get(1)?;
    Ok(repository::Tag { id, title })
}

#[cfg(test)]
mod create_tag_tests {
    use crate::model::repository::Tag;
    use crate::repository::{open_connection, tag_repository};
    use crate::test::refresh_db;

    #[test]
    fn create_tag() {
        refresh_db();
        let con = open_connection();
        let tag = tag_repository::create_tag("test".to_string(), &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test".to_string()
            },
            tag
        );
    }
}

#[cfg(test)]
mod get_tag_by_title_tests {
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, get_tag_by_title};
    use crate::test::refresh_db;

    #[test]
    fn get_tag_by_title_found() {
        refresh_db();
        let con = open_connection();
        create_tag("test".to_string(), &con).unwrap();
        let found = get_tag_by_title("TeSt".to_string(), &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Some(Tag {
                id: 1,
                title: "test".to_string()
            }),
            found
        );
    }

    #[test]
    fn get_tag_by_title_not_found() {
        refresh_db();
        let con = open_connection();
        let not_found = get_tag_by_title("test".to_string(), &con).unwrap();
        con.close().unwrap();
        assert_eq!(None, not_found);
    }
}

#[cfg(test)]
mod get_tag_by_id_tests {
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, get_tag};
    use crate::test::refresh_db;

    #[test]
    fn get_tag_success() {
        refresh_db();
        let con = open_connection();
        create_tag("test".to_string(), &con).unwrap();
        let tag = get_tag(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test".to_string()
            },
            tag
        );
    }
}

#[cfg(test)]
mod update_tag_tests {
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, get_tag, update_tag};
    use crate::test::refresh_db;

    #[test]
    fn update_tag_success() {
        refresh_db();
        let con = open_connection();
        create_tag("test".to_string(), &con).unwrap();
        update_tag(
            Tag {
                id: 1,
                title: "test2".to_string(),
            },
            &con,
        )
        .unwrap();
        let res = get_tag(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test2".to_string()
            },
            res
        );
    }
}

#[cfg(test)]
mod delete_tag_tests {
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, delete_tag, get_tag};
    use crate::test::refresh_db;

    #[test]
    fn delete_tag_success() {
        refresh_db();
        let con = open_connection();
        create_tag("test".to_string(), &con).unwrap();
        delete_tag(1, &con).unwrap();
        let not_found = get_tag(1, &con);
        con.close().unwrap();
        assert_eq!(Err(rusqlite::Error::QueryReturnedNoRows), not_found);
    }
}
