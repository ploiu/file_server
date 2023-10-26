use core::option::Option;

use regex::Regex;
use rocket::fs::TempFile;
use rocket::serde::{Deserialize, Serialize};
use crate::model::response::Tag;

#[derive(FromForm)]
#[allow(non_snake_case)] // cannot serde rename the field, and it's better to have camel case for the api
pub struct CreateFileRequest<'a> {
    /// the file being uploaded
    pub file: TempFile<'a>,
    /// because I don't feel like mapping from content-type header
    pub extension: Option<String>,
    /// leave blank for top level folder
    ///
    /// so it _appears_ that Rocket has trouble parsing form data fields if they're a number,
    /// and turning it into a String fixes it.
    /// Weird thing is, it works from postman and curl, but not from javascript form body,
    /// intellij http scratch pad (even directly imported from curl), or java.
    /// I don't want to pursue this anymore, and this works
    folderId: Option<String>,
}

impl CreateFileRequest<'_> {
    pub fn folder_id(&self) -> u32 {
        match &self.folderId {
            Some(id) => id.to_string().parse::<u32>(),
            None => Ok(0),
        }
        .unwrap()
    }
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateFileRequest {
    pub id: u32,
    #[serde(rename = "folderId")]
    pub folder_id: Option<u32>,
    /// this value may be unsafe, see [`UpdateFileRequest::name`]
    name: String,
    tags: Vec<Tag>
}

impl UpdateFileRequest {
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
            || *&self.name.starts_with("..")
            || *&self.name.contains("./")
        {
            return None;
        }
        let replaced = banned_chars.replace_all(&self.name, "");
        let replaced = replaced
            .to_string()
            .replace("(", "leftParenthese")
            .replace(")", "rightParenthese");
        Some(replaced)
    }

    // this gets warned as dead code, but it has a ton of usage. maybe the rust foundation should fix this instead of playing politics
    #[cfg(test)]
    pub fn new(id: u32, folder_id: Option<u32>, name: String) -> UpdateFileRequest {
        UpdateFileRequest {
            id,
            folder_id,
            name,
            tags: Vec::new()
        }
    }
}

#[cfg(test)]
mod update_file_request_tests {
    use crate::model::request::file_requests::UpdateFileRequest;

    fn fail() {
        assert!(false)
    }

    #[test]
    fn name_removes_invalid_names() {
        let invalid_names = vec![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ];
        for name in invalid_names.iter() {
            let req = UpdateFileRequest::new(1, None, name.to_string());
            println!("Testing {}", name);
            assert_eq!(None, req.name());
        }
    }

    #[test]
    fn name_keeps_file_extension() {
        let req = UpdateFileRequest::new(1, None, "test.txt".to_string());
        assert_eq!("test.txt".to_string(), req.name().unwrap());
    }

    // files that are only extensions (like .bashrc) are allowed
    #[test]
    fn name_keeps_files_with_only_extension() {
        let req = UpdateFileRequest::new(1, None, ".bashrc".to_string());
        assert_eq!(".bashrc".to_string(), req.name().unwrap());
    }

    #[test]
    fn name_replaces_parentheses() {
        let req = UpdateFileRequest::new(1, None, "test (1).txt".to_string());
        assert_eq!(
            "test leftParenthese1rightParenthese.txt".to_string(),
            req.name().unwrap()
        );
    }

    #[test]
    fn name_keeps_multiple_extensions() {
        let req = UpdateFileRequest::new(1, None, "test.old.txt.bak".to_string());
        assert_eq!("test.old.txt.bak".to_string(), req.name().unwrap());
    }

    #[test]
    fn name_removes_path_traversal_attempts() {
        let req = UpdateFileRequest::new(1, None, "./folders/y.txt".to_string());
        assert_eq!(None, req.name());
        let req = UpdateFileRequest::new(1, None, "../whatever/a.txt".to_string());
        assert_eq!(None, req.name());
    }
}
