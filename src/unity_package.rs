use flate2::read::GzDecoder;
use rust_tools::prelude::*;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use tar::Archive;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum UnityPackageReaderError {
    PackageNotFound,
    CorruptPackage,
    DirectoryCouldNotBeCreated,
    WorkingDirectoryError,
    PathError,
}

impl fmt::Display for UnityPackageReaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnityPackageReaderError::PackageNotFound => write!(f, "File not found"),
            UnityPackageReaderError::PathError => write!(f, "Could not create a valid path."),
            UnityPackageReaderError::CorruptPackage => write!(f, "Could not unpack a unity package, it seems to be corrupt."),
            UnityPackageReaderError::DirectoryCouldNotBeCreated => write!(f, "Could not create the temporary directory to extract files to."),
            UnityPackageReaderError::WorkingDirectoryError => write!(f, "Could not determine the current working directory. Consider passing an absolute path to the file to create a UnityPackage."),
        }
    }
}

pub struct UnityPackage {
    path: String,
}

impl UnityPackage {
    /// Creates a new UnityPackage. The given file name is either the absolute path
    /// to the package on disk or the name of the file in the current
    /// working directory (or a subdirectory of the current working directory).
    pub fn new(file_name: &str) -> Result<Self, UnityPackageReaderError> {
        let p = Path::new(file_name);

        let mut path = String::from(file_name);
        if !p.exists() {
            if let Ok(mut working_dir) = std::env::current_dir() {
                working_dir.push(file_name);
                path = match working_dir.into_os_string().into_string() {
                    Ok(p) => p,
                    Err(_) => return Err(UnityPackageReaderError::PathError),
                }
            } else {
                return Err(UnityPackageReaderError::WorkingDirectoryError);
            }
        }

        Ok(UnityPackage { path })
    }

    /// The default tmp directory is always the current [working directory]/tmp
    fn get_default_tmp_dir() -> Result<PathBuf, UnityPackageReaderError> {
        if let Ok(mut working_dir) = std::env::current_dir() {
            working_dir.push("tmp");
            Ok(working_dir)
        } else {
            Err(UnityPackageReaderError::WorkingDirectoryError)
        }
    }

    pub fn unpack_package(
        &self,
        extract_to: Option<&Path>,
    ) -> Result<PathBuf, UnityPackageReaderError> {
        let tmp = get_file_as_byte_vec(Path::new(self.path.clone().as_str()));
        match tmp {
            Ok(bytes) => {
                let path = match extract_to {
                    Some(t) => PathBuf::from(t),
                    None => match Self::get_default_tmp_dir() {
                        Ok(e) => e,
                        Err(e) => {
                            return Err(e);
                        }
                    },
                };

                let tar = GzDecoder::new(&bytes[..]);
                let mut archive = Archive::new(tar);

                match std::fs::create_dir_all(path.clone()) {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(UnityPackageReaderError::DirectoryCouldNotBeCreated);
                    }
                }

                match archive.unpack(path.clone()) {
                    Ok(_) => {
                        let r = path;
                        Ok(r)
                    }
                    Err(_) => Err(UnityPackageReaderError::CorruptPackage),
                }
            }
            Err(e) => match e {
                FileErrors::FileNotFound => Err(UnityPackageReaderError::PackageNotFound),
                FileErrors::CorruptFile => Err(UnityPackageReaderError::CorruptPackage),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tmp_dir() {
        let mut p = std::env::current_dir().unwrap();
        p.push("tmp");

        assert_eq!(p, UnityPackage::get_default_tmp_dir().unwrap());
    }

    #[test]
    fn test_new_function_with_file_name() {
        let n = "file_name.unitypackage";
        let mut p = std::env::current_dir().unwrap();
        p.push(n);

        let package = UnityPackage::new(n).unwrap();

        assert_eq!(p.into_os_string().into_string().unwrap(), package.path)
    }

    #[test]
    fn test_new_function_with_path() {
        let mut p = std::env::current_dir().unwrap();
        let parent = match p.parent() {
            Some(i) => i,
            None => {
                panic!("Could not determine path")
            }
        };

        p = parent.to_path_buf();
        p.push("file_name.unitypackage");

        let subject =
            UnityPackage::new(p.clone().into_os_string().into_string().unwrap().as_str()).unwrap();
        assert_eq!(p.into_os_string().into_string().unwrap(), subject.path);
    }
}
