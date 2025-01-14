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

impl Default for FileTypes {
    fn default() -> Self {
        Self::Unknown
    }
}
