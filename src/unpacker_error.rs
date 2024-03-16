use std::fmt;

#[derive(Debug, PartialEq, PartialOrd)]
pub struct ErrorInformation {
    pub message: Option<String>,
    pub src_file: String,
    pub line_no: u32,
}

impl ErrorInformation {
    pub fn new(message: Option<String>, src_file: &str, line_no: u32) -> Self {
        ErrorInformation {
            message,
            src_file: String::from(src_file),
            line_no,
        }
    }
}

impl fmt::Display for ErrorInformation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.message {
            Some(s) => write!(f, "{}, {}, {}", s, self.src_file, self.line_no),
            None => {
                write!(f, "in: '{}' line: {}", self.src_file, self.line_no)
            }
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum UnityPackageReaderError {
    PackageNotFound(ErrorInformation),
    CorruptPackage(ErrorInformation),
    TmpDirectoryCouldNotBeCreated(ErrorInformation),
    TargetDirectoryCouldNotBeCreated(ErrorInformation),
    WorkingDirectoryError(ErrorInformation),
    PathError(ErrorInformation),
    NotAPackageFile(ErrorInformation),
    CouldReadMetaFile(ErrorInformation),
    CouldNotDeleteTmp(ErrorInformation),
}

impl fmt::Display for UnityPackageReaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnityPackageReaderError::PackageNotFound(e) => write!(f, "File not found.\n{}", e),
            UnityPackageReaderError::PathError(e) => write!(f, "Could not create a valid path.\n{}", e),
            UnityPackageReaderError::CorruptPackage(s) => write!(f, "Could not unpack a unity package.\n{}", s),
            UnityPackageReaderError::TmpDirectoryCouldNotBeCreated(s) => write!(f, "Could not create the temp dir.\n{}", s),
            UnityPackageReaderError::TargetDirectoryCouldNotBeCreated(s) => write!(f, "Could not create the target dir.\n{}", s),
            UnityPackageReaderError::WorkingDirectoryError(e) => write!(f, "Could not determine the current working directory. Consider passing an absolute path to the file to create a UnityPackage.{}", e),
            UnityPackageReaderError::NotAPackageFile(e) => write!(f, "The given path seems to point to a directory.{}", e),
            UnityPackageReaderError::CouldReadMetaFile(e) => write!(f, "Could not interpret meta data.{}", e),
            UnityPackageReaderError::CouldNotDeleteTmp(e) => write!(f, "Could not delete tmp directory.{}", e),
        }
    }
}
