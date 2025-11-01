
    use crate::test::{cleanup, refresh_db};

    #[test]
    fn generate_preview_successfully_creates_preview_for_file() {
        refresh_db();
        crate::fail!();
        cleanup();
    }

    #[test]
    fn generate_preview_ignores_missing_file_from_db() {
        refresh_db();
        crate::fail!();
        cleanup();
    }

    #[test]
    fn generate_preview_no_ffmpeg() {
        refresh_db();
        crate::fail!();
        cleanup();
    }

    #[test]
    fn generate_preview_message_not_file_id() {
        refresh_db();
        crate::fail!();
        cleanup();
    }

    #[test]
    fn generate_preview_ignores_missing_file_from_disk() {
        refresh_db();
        crate::fail!();
        cleanup();
    }

    #[test]
    fn generate_preview_generates_for_image() {
        refresh_db();
        crate::fail!();
        cleanup();
    }

    #[test]
    fn generate_preview_generates_for_video() {
        refresh_db();
        crate::fail!();
        cleanup();
    }

    #[test]
    fn generate_preview_does_not_generate_for_other_file_types() {
        refresh_db();
        crate::fail!();
        cleanup();
    }
