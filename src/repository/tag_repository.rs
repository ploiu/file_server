use rusqlite::Connection;

use crate::model::repository;

/// creates a new tag in the database. This does not check if the tag already exists,
/// so the caller must check that themselves
pub fn create_tag(title: &String, con: &Connection) -> Result<repository::Tag, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/create_tag.sql"))?;
    let id = pst.insert(rusqlite::params![title])? as u32;
    Ok(repository::Tag {
        id,
        title: title.clone(),
    })
}

/// searches for a tag that case-insensitively matches that passed title.
///
/// if `None` is returned, that means there was no match
pub fn get_tag_by_title(
    title: &String,
    con: &Connection,
) -> Result<Option<repository::Tag>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/get_by_title.sql"))?;
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
    let mut pst = con.prepare(include_str!("../assets/queries/tags/get_by_id.sql"))?;
    return Ok(pst.query_row(rusqlite::params![id], tag_mapper)?);
}

/// updates the past tag. Checking to make sure the tag exists needs to be done on the caller's end
pub fn update_tag(tag: repository::Tag, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/update_tag.sql"))?;
    pst.execute(rusqlite::params![tag.title, tag.id])?;
    Ok(())
}

pub fn delete_tag(id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/delete_tag.sql"))?;
    pst.execute(rusqlite::params![id])?;
    Ok(())
}

/// the caller of this function will need to make sure the tag already exists and isn't already on the file
pub fn add_tag_to_file(file_id: u32, tag_id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/add_tag_to_file.sql"))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

pub fn get_tags_on_file(
    file_id: u32,
    con: &Connection,
) -> Result<Vec<repository::Tag>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/get_tags_for_file.sql"))?;
    let rows = pst.query_map(rusqlite::params![file_id], |row| Ok(tag_mapper(&row)))?;
    let mut tags: Vec<repository::Tag> = Vec::new();
    for tag_res in rows {
        // I know it's probably bad style, but I'm laughing too hard at the double question mark.
        // no I don't know what my code is doing and I'm glad my code reflects that
        tags.push(tag_res??);
    }
    Ok(tags)
}

pub fn remove_tag_from_file(
    file_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_tag_from_file.sql"
    ))?;
    pst.execute(rusqlite::params![file_id, tag_id])?;
    Ok(())
}

pub fn add_tag_to_folder(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!("../assets/queries/tags/add_tag_to_folder.sql"))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
    Ok(())
}

pub fn get_tags_on_folder(
    folder_id: u32,
    con: &Connection,
) -> Result<Vec<repository::Tag>, rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/get_tags_for_folder.sql"
    ))?;
    let rows = pst.query_map(rusqlite::params![folder_id], |row| Ok(tag_mapper(&row)))?;
    let mut tags: Vec<repository::Tag> = Vec::new();
    for tag_res in rows {
        // I know it's probably bad style, but I'm laughing too hard at the double question mark.
        // no I don't know what my code is doing and I'm glad my code reflects that
        tags.push(tag_res??);
    }
    Ok(tags)
}

pub fn remove_tag_from_folder(
    folder_id: u32,
    tag_id: u32,
    con: &Connection,
) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/tags/remove_tag_from_folder.sql"
    ))?;
    pst.execute(rusqlite::params![folder_id, tag_id])?;
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
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn create_tag() {
        refresh_db();
        let con = open_connection();
        let tag = tag_repository::create_tag(&"test".to_string(), &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test".to_string(),
            },
            tag
        );
        cleanup();
    }
}

#[cfg(test)]
mod get_tag_by_title_tests {
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, get_tag_by_title};
    use crate::test::*;

    #[test]
    fn get_tag_by_title_found() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
        let found = get_tag_by_title(&"TeSt".to_string(), &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Some(Tag {
                id: 1,
                title: "test".to_string(),
            }),
            found
        );
        cleanup();
    }
    #[test]
    fn get_tag_by_title_not_found() {
        refresh_db();
        let con = open_connection();
        let not_found = get_tag_by_title(&"test".to_string(), &con).unwrap();
        con.close().unwrap();
        assert_eq!(None, not_found);
        cleanup();
    }
}

#[cfg(test)]
mod get_tag_by_id_tests {
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, get_tag};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn get_tag_success() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
        let tag = get_tag(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            Tag {
                id: 1,
                title: "test".to_string(),
            },
            tag
        );
        cleanup();
    }
}

#[cfg(test)]
mod update_tag_tests {
    use crate::model::repository::Tag;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, get_tag, update_tag};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn update_tag_success() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
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
                title: "test2".to_string(),
            },
            res
        );
        cleanup();
    }
}

#[cfg(test)]
mod delete_tag_tests {
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, delete_tag, get_tag};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn delete_tag_success() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
        delete_tag(1, &con).unwrap();
        let not_found = get_tag(1, &con);
        con.close().unwrap();
        assert_eq!(Err(rusqlite::Error::QueryReturnedNoRows), not_found);
        cleanup();
    }
}

#[cfg(test)]
mod get_tag_on_file_tests {
    use crate::model::repository::{FileRecord, Tag};
    use crate::repository::file_repository::create_file;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{add_tag_to_file, create_tag, get_tags_on_file};
    use crate::test::*;

    #[test]
    fn get_tags_on_file_returns_tags() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
        create_tag(&"test2".to_string(), &con).unwrap();
        create_file(
            &FileRecord {
                id: None,
                name: "test_file".to_string(),
                parent_id: None,
            },
            &con,
        )
        .unwrap();
        add_tag_to_file(1, 1, &con).unwrap();
        add_tag_to_file(1, 2, &con).unwrap();
        let res = get_tags_on_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            vec![
                Tag {
                    id: 1,
                    title: "test".to_string()
                },
                Tag {
                    id: 2,
                    title: "test2".to_string()
                }
            ],
            res
        );
        cleanup();
    }
    #[test]
    fn get_tags_on_file_returns_nothing_if_no_tags() {
        refresh_db();
        let con = open_connection();
        create_file(
            &FileRecord {
                id: None,
                name: "test_file".to_string(),
                parent_id: None,
            },
            &con,
        )
        .unwrap();
        let res = get_tags_on_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), res);
        cleanup();
    }
}

#[cfg(test)]
mod remove_tag_from_file_tests {
    use crate::model::repository::{FileRecord, Tag};
    use crate::repository::file_repository::create_file;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{create_tag, get_tags_on_file, remove_tag_from_file};
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn remove_tag_from_file_works() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
        create_file(
            &FileRecord {
                id: None,
                name: "test_file".to_string(),
                parent_id: None,
            },
            &con,
        )
        .unwrap();
        remove_tag_from_file(1, 1, &con).unwrap();
        let tags = get_tags_on_file(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), tags);
        cleanup();
    }
}

#[cfg(test)]
mod get_tag_on_folder_tests {
    use crate::model::repository::{Folder, Tag};
    use crate::repository::folder_repository::create_folder;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{add_tag_to_folder, create_tag, get_tags_on_folder};
    use crate::test::*;

    #[test]
    fn get_tags_on_folder_returns_tags() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
        create_tag(&"test2".to_string(), &con).unwrap();
        create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        add_tag_to_folder(1, 1, &con).unwrap();
        add_tag_to_folder(1, 2, &con).unwrap();
        let res = get_tags_on_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(
            vec![
                Tag {
                    id: 1,
                    title: "test".to_string()
                },
                Tag {
                    id: 2,
                    title: "test2".to_string()
                }
            ],
            res
        );
        cleanup();
    }
    #[test]
    fn get_tags_on_folder_returns_nothing_if_no_tags() {
        refresh_db();
        let con = open_connection();
        create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        let res = get_tags_on_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), res);
        cleanup();
    }
}

#[cfg(test)]
mod remove_tag_from_folder_tests {
    use crate::model::repository::{Folder, Tag};
    use crate::repository::folder_repository::create_folder;
    use crate::repository::open_connection;
    use crate::repository::tag_repository::{
        create_tag, get_tags_on_folder, remove_tag_from_folder,
    };
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn remove_tag_from_folder_works() {
        refresh_db();
        let con = open_connection();
        create_tag(&"test".to_string(), &con).unwrap();
        create_folder(
            &Folder {
                parent_id: None,
                id: None,
                name: "test_folder".to_string(),
            },
            &con,
        )
        .unwrap();
        remove_tag_from_folder(1, 1, &con).unwrap();
        let tags = get_tags_on_folder(1, &con).unwrap();
        con.close().unwrap();
        assert_eq!(Vec::<Tag>::new(), tags);
        cleanup();
    }
}
