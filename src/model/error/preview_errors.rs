pub enum PreviewError {
    /// The image failed to be opened
    Open,
    /// The image could not be decoded properly
    Decode,
    /// The image could not be encoded properly
    Encode,
    /// The image format could not be guessed
    GuessFormat,
}
