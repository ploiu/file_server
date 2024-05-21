pub enum PreviewError {
    /// The image failed to be opened
    FailedOpen,
    /// The image could not be decoded properly
    FailedDecode,
    /// The image could not be encoded properly
    FailedEncode,
    /// The image format could not be guessed
    FailedGuessFormat,
}
