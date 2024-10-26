
use chrono::NaiveDateTime;
use regex::Regex;
use rocket::serde::{Deserialize, Serialize};
use rusqlite::ToSql;

use crate::model::repository::FileRecord;
use crate::model::response::TagApi;

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum FileTypes {
    Application,
    Archive,
    Audio,
    Cad,
    Code,
    Configuration,
    Diagram,
    Document,
    Font,
    GameRom,
    Image,
    Material,
    Model,
    Object,
    Presentation,
    SaveFile,
    Spreadsheet,
    Text,
    Video,
    Unknown,
}

impl From<String> for FileTypes {
    fn from(value: String) -> Self {
        let value = value.as_str();
        match value.to_ascii_lowercase().as_str() {
            "application" => Self::Application,
            "archive" => Self::Archive,
            "audio" => Self::Audio,
            "cad" => Self::Cad,
            "code" => Self::Code,
            "configuration" => Self::Configuration,
            "diagram" => Self::Diagram,
            "document" => Self::Document,
            "font" => Self::Font,
            "game_rom" => Self::GameRom,
            "image" => Self::Image,
            "material" => Self::Material,
            "model" => Self::Model,
            "object" => Self::Object,
            "presentation" => Self::Presentation,
            "save_file" => Self::SaveFile,
            "spreadsheet" => Self::Spreadsheet,
            "text" => Self::Text,
            "video" => Self::Video,
            "unknown" => Self::Unknown,
            _ => {
                log::warn!(
                    "file type from database {value} does not match any branches in FileTypes#from"
                );
                Self::Unknown
            }
        }
    }
}

impl ToSql for FileTypes {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Self::Application => Ok("application".into()),
            Self::Archive => Ok("archive".into()),
            Self::Audio => Ok("audio".into()),
            Self::Cad => Ok("cad".into()),
            Self::Code => Ok("code".into()),
            Self::Configuration => Ok("configuration".into()),
            Self::Diagram => Ok("diagram".into()),
            Self::Document => Ok("document".into()),
            Self::Font => Ok("font".into()),
            Self::GameRom => Ok("game_rom".into()),
            Self::Image => Ok("image".into()),
            Self::Material => Ok("material".into()),
            Self::Model => Ok("model".into()),
            Self::Object => Ok("object".into()),
            Self::Presentation => Ok("presentation".into()),
            Self::SaveFile => Ok("save_file".into()),
            Self::Spreadsheet => Ok("spreadsheet".into()),
            Self::Text => Ok("text".into()),
            Self::Video => Ok("video".into()),
            Self::Unknown => Ok("unknown".into()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(crate = "rocket::serde")]
pub struct FileMetadata {
    pub size: u32,
    pub date_created: u64,
    pub file_type: FileTypes,
}

#[derive(Deserialize, Serialize, Debug, Hash, Clone, Eq)]
#[cfg_attr(not(test), derive(PartialEq))]
#[serde(crate = "rocket::serde")]
pub struct FileApi {
    pub id: u32,
    // I can revisit including this in the response later, but for now it's out of scope
    #[serde(rename = "folderId", skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<u32>,
    /// this value may be unsafe, see [`FileApi::name`]
    pub name: String,
    pub tags: Vec<TagApi>,
    // wrapped in option so api consumers don't have to send this field (these fields can't be written to after a file is uploaded)
    pub size: Option<u64>,
    pub create_date: Option<NaiveDateTime>,
    pub file_type: Option<FileTypes>,
}

impl FileApi {
    /// returns a sanitized string based on [Rocket's file name sanitization](https://api.rocket.rs/master/rocket/fs/struct.FileName.html#sanitization)
    /// but with the exception of parentheses being replaced with `leftParenthese` and `rightParenthese` respectively. It's hacky, but parentheses in file
    /// names are super common and don't immediately mean it's malicious
    /// will return None if the entire file name is unsafe
    pub fn name(&self) -> Option<String> {
        //language=RegExp
        let reserved_name_regex = Regex::new("^CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9]$").unwrap();
        //language=RegExp
        let banned_chars = Regex::new("(^\\.\\.|^\\./)|[/\\\\<>|:&;#?*]").unwrap();
        if reserved_name_regex.is_match(&self.name.to_uppercase())
            || self.name.starts_with("..")
            || self.name.contains("./")
        {
            return None;
        }
        let replaced = banned_chars.replace_all(&self.name, "");
        let replaced = replaced
            .to_string()
            .replace('(', "leftParenthese")
            .replace(')', "rightParenthese");
        Some(replaced)
    }

    pub fn from(file: FileRecord, tags: Vec<TagApi>) -> FileApi {
        FileApi {
            tags,
            id: file.id.unwrap(),
            folder_id: file.parent_id,
            name: file.name,
            size: Some(file.size),
            create_date: Some(file.create_date),
            // TODO file_types
            file_type: None,
        }
    }

    #[cfg(test)]
    pub fn new(id: u32, folder_id: Option<u32>, name: String) -> FileApi {
        FileApi {
            id,
            folder_id,
            name,
            tags: Vec::new(),
            size: None,
            create_date: None,
            // TODO file_types
            file_type: None,
        }
    }
}

#[cfg(test)]
mod update_file_request_tests {
    use crate::model::api::FileApi;

    #[test]
    fn name_removes_invalid_names() {
        let invalid_names = vec![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ];
        for name in invalid_names.iter() {
            let req = FileApi::new(1, None, name.to_string());
            println!("Testing {}", name);
            assert_eq!(None, req.name());
        }
    }

    #[test]
    fn name_keeps_file_extension() {
        let req = FileApi::new(1, None, "test.txt".to_string());
        assert_eq!("test.txt".to_string(), req.name().unwrap());
    }

    // files that are only extensions (like .bashrc) are allowed
    #[test]
    fn name_keeps_files_with_only_extension() {
        let req = FileApi::new(1, None, ".bashrc".to_string());
        assert_eq!(".bashrc".to_string(), req.name().unwrap());
    }

    #[test]
    fn name_replaces_parentheses() {
        let req = FileApi::new(1, None, "test (1).txt".to_string());
        assert_eq!(
            "test leftParenthese1rightParenthese.txt".to_string(),
            req.name().unwrap()
        );
    }

    #[test]
    fn name_keeps_multiple_extensions() {
        let req = FileApi::new(1, None, "test.old.txt.bak".to_string());
        assert_eq!("test.old.txt.bak".to_string(), req.name().unwrap());
    }

    #[test]
    fn name_removes_path_traversal_attempts() {
        let req = FileApi::new(1, None, "./folders/y.txt".to_string());
        assert_eq!(None, req.name());
        let req = FileApi::new(1, None, "../whatever/a.txt".to_string());
        assert_eq!(None, req.name());
    }
}
