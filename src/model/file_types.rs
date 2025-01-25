use std::fmt::Display;

use rusqlite::ToSql;
use serde::{Deserialize, Serialize};

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
    Rom,
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

impl From<&str> for FileTypes {
    fn from(value: &str) -> Self {
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
            "rom" => Self::Rom,
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
            Self::Rom => Ok("rom".into()),
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

impl Display for FileTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Application => write!(f, "application"),
            Self::Archive => write!(f, "archive"),
            Self::Audio => write!(f, "audio"),
            Self::Cad => write!(f, "cad"),
            Self::Code => write!(f, "code"),
            Self::Configuration => write!(f, "configuration"),
            Self::Diagram => write!(f, "diagram"),
            Self::Document => write!(f, "document"),
            Self::Font => write!(f, "font"),
            Self::Rom => write!(f, "rom"),
            Self::Image => write!(f, "image"),
            Self::Material => write!(f, "material"),
            Self::Model => write!(f, "model"),
            Self::Object => write!(f, "object"),
            Self::Presentation => write!(f, "presentation"),
            Self::SaveFile => write!(f, "save_file"),
            Self::Spreadsheet => write!(f, "spreadsheet"),
            Self::Text => write!(f, "text"),
            Self::Video => write!(f, "video"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl Default for FileTypes {
    fn default() -> Self {
        Self::Unknown
    }
}
