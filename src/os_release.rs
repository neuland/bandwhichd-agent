#[derive(Clone)]
pub struct OsRelease {
    file_contents: String,
}

impl OsRelease {
    pub fn new(file_contents: String) -> Self {
        OsRelease { file_contents }
    }

    pub fn read() -> Result<OsRelease, failure::Error> {
        let file_contents = std::fs::read_to_string("/etc/os-release")?;
        Ok(OsRelease::new(file_contents))
    }

    pub fn file_contents(&self) -> String {
        self.file_contents.clone()
    }
}
