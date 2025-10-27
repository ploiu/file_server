use rusqlite::{Connection, params};

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

pub fn delete_file_preview(file_id: u32, con: &Connection) -> Result<(), rusqlite::Error> {
    let mut pst = con.prepare(include_str!(
        "../assets/queries/file/delete_file_preview.sql"
    ))?;
    pst.execute(params![file_id])?;
    Ok(())
}

pub fn get_file_preview(file_id: u32, con: &Connection) -> Result<Vec<u8>, rusqlite::Error> {
    let mut pst = con.prepare(&format!(
        include_str!("../assets/queries/file/get_file_preview.sql"),
        file_id
    ))?;
    let res: Vec<u8> = pst.query_row([], |row| row.get(0))?;
    Ok(res)
}

#[cfg(test)]
mod file_preview_tests {
    use rusqlite::Connection;

    use super::*;

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
