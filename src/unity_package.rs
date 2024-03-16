use flate2::read::GzDecoder;
use rust_tools::prelude::*;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tar::Archive;

use crate::{
    prelude::UnityAssetFile,
    unpacker_error::{ErrorInformation, UnityPackageReaderError},
};

pub struct UnityPackage {
    /// The name of the file to unpack.
    path: String,
    /// The target directory. If none is set the current working directory and the name of the package will be used
    target_path: Option<String>,
    /// We have to unpack the file into a tmp directory
    temp_directory: Option<String>,
    /// The files we found hashed by the guid
    files: HashMap<String, UnityAssetFile>,
}

impl UnityPackage {
    /// Creates a new UnityPackage. The given file name is either the absolute path
    /// to the package on disk or the name of the file in the current
    /// working directory (or a subdirectory of the current working directory).
    pub fn new(
        file_name: &str,
        target_path: Option<String>,
        temp_directory: Option<String>,
    ) -> Result<Self, UnityPackageReaderError> {
        let p = Path::new(file_name);

        let mut path = String::from(file_name);
        if !p.exists() {
            if let Ok(mut working_dir) = std::env::current_dir() {
                working_dir.push(file_name);
                path = match working_dir.into_os_string().into_string() {
                    Ok(p) => p,
                    Err(e) => {
                        return Err(UnityPackageReaderError::PathError(ErrorInformation::new(
                            Some(format!("{:?}", e)),
                            file!(),
                            line!(),
                        )))
                    }
                }
            } else {
                return Err(UnityPackageReaderError::WorkingDirectoryError(
                    ErrorInformation::new(None, file!(), line!()),
                ));
            }
        }

        Ok(UnityPackage {
            path,
            target_path,
            temp_directory,
            files: HashMap::new(),
        })
    }

    pub fn get_path(&self) -> String {
        self.path.clone()
    }

    pub fn get_file(&self, guid: &String) -> Option<&UnityAssetFile> {
        self.files.get(guid)
    }

    /// The default tmp directory is always the current [working directory]/tmp
    pub fn get_tmp_dir(&self) -> Result<PathBuf, UnityPackageReaderError> {
        match &self.temp_directory {
            Some(s) => Ok(PathBuf::from(s)),
            None => {
                if let Ok(mut working_dir) = std::env::current_dir() {
                    working_dir.push("tmp");
                    Ok(working_dir)
                } else {
                    Err(UnityPackageReaderError::WorkingDirectoryError(
                        ErrorInformation::new(None, file!(), line!()),
                    ))
                }
            }
        }
    }

    /// Return the file name of the package without extension.
    fn get_package_file_name(&self) -> Result<String, UnityPackageReaderError> {
        let p = PathBuf::from(&self.path);

        match p.file_stem() {
            Some(s) => {
                if let Some(file_stem) = s.to_str() {
                    Ok(String::from(file_stem))
                } else {
                    Err(UnityPackageReaderError::NotAPackageFile(
                        ErrorInformation::new(None, file!(), line!()),
                    ))
                }
            }
            None => Err(UnityPackageReaderError::NotAPackageFile(
                ErrorInformation::new(None, file!(), line!()),
            )),
        }
    }

    /// Get the target directory. If the target has been set by the user
    /// then this directory is beeing return.
    /// Otherwise we use the current working directory and append the file name
    /// of the package.
    pub fn get_target_dir(&self) -> Result<PathBuf, UnityPackageReaderError> {
        match &self.target_path {
            Some(s) => Ok(PathBuf::from(s)),

            None => match self.get_package_file_name() {
                Ok(s) => match std::env::current_dir() {
                    Ok(mut r) => {
                        r.push(s);
                        Ok(r)
                    }
                    Err(e) => Err(UnityPackageReaderError::WorkingDirectoryError(
                        ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                    )),
                },
                Err(e) => Err(UnityPackageReaderError::NotAPackageFile(
                    ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                )),
            },
        }
    }

    pub fn unpack_package(&mut self, delete_tmp: bool) -> Result<(), UnityPackageReaderError> {
        let tmp = get_file_as_byte_vec(Path::new(self.path.clone().as_str()));
        match tmp {
            Ok(bytes) => {
                let tar = GzDecoder::new(&bytes[..]);
                let mut archive = Archive::new(tar);

                let tmp_path = match self.get_tmp_dir() {
                    Ok(e) => e,
                    Err(e) => {
                        return Err(UnityPackageReaderError::TmpDirectoryCouldNotBeCreated(
                            ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                        ));
                    }
                };

                match std::fs::create_dir_all(tmp_path.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(UnityPackageReaderError::TmpDirectoryCouldNotBeCreated(
                            ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                        ));
                    }
                }

                match archive.unpack(tmp_path.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(UnityPackageReaderError::CorruptPackage(
                            ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                        ));
                    }
                }

                match self.copy_files_to_target() {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(e);
                    }
                }

                if delete_tmp {
                    match std::fs::remove_dir_all(tmp_path) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(UnityPackageReaderError::CouldNotDeleteTmp(
                            ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                        )),
                    }
                } else {
                    Ok(())
                }
            }

            Err(e) => match e {
                FileErrors::FileNotFound => Err(UnityPackageReaderError::PackageNotFound(
                    ErrorInformation::new(None, file!(), line!()),
                )),
                FileErrors::CorruptFile => Err(UnityPackageReaderError::CorruptPackage(
                    ErrorInformation::new(None, file!(), line!()),
                )),
            },
        }
    }

    fn copy_files_to_target(&mut self) -> Result<(), UnityPackageReaderError> {
        let p = self.get_tmp_dir();
        let t = self.get_target_dir();

        let target = match t {
            Ok(f) => f,
            Err(e) => return Err(e),
        };

        let origin = match p {
            Ok(f) => f,
            Err(e) => return Err(e),
        };

        let files = match fs::read_dir(origin.clone()) {
            Ok(f) => f,
            Err(e) => {
                return Err(UnityPackageReaderError::TmpDirectoryCouldNotBeCreated(
                    ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                ));
            }
        };

        for entry in files {
            let entry = match entry {
                Ok(f) => f,
                Err(e) => {
                    return Err(UnityPackageReaderError::CorruptPackage(
                        ErrorInformation::new(Some(format!("{}", e)), file!(), line!()),
                    ))
                }
            };

            let p = entry.path();
            let asset_file = UnityAssetFile::from(p);
            match asset_file {
                Ok(mut a) => {
                    match a.copy_asset(&target) {
                        Ok(()) => {}
                        Err(e) => {
                            return Err(e);
                        }
                    }
                    self.files.insert(a.get_guid().clone(), a);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use super::*;

    fn get_test_base_path() -> PathBuf {
        let mut r = std::env::current_dir().unwrap();
        if r.ends_with("unity_unpacker_lib") {
            r = r.parent().unwrap().to_path_buf();
        }

        r
    }

    #[test]
    fn test_default_tmp_dir() {
        let mut p = std::env::current_dir().unwrap();
        p.push("tmp");

        let mut t2 = std::env::current_dir().unwrap();
        t2.push("file");

        let item = UnityPackage::new("file.unitypackage", None, None).unwrap();

        assert_eq!(p, item.get_tmp_dir().unwrap());
        assert_eq!(item.get_target_dir().unwrap(), t2);
    }

    #[test]
    fn test_new_function_with_file_name() {
        let n = "file_name.unitypackage";
        let mut p = std::env::current_dir().unwrap();
        p.push(n);

        let mut t2 = std::env::current_dir().unwrap();
        t2.push("file_name");

        let package = UnityPackage::new(n, None, None).unwrap();

        assert_eq!(p.into_os_string().into_string().unwrap(), package.path);
        assert_eq!(package.get_target_dir().unwrap(), t2);
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

        let mut t2 = std::env::current_dir().unwrap();
        t2.push("file_name");

        let subject = UnityPackage::new(
            p.clone().into_os_string().into_string().unwrap().as_str(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(p.into_os_string().into_string().unwrap(), subject.path);
        assert_eq!(subject.get_target_dir().unwrap(), t2);
    }

    #[test]
    fn test_new_function_with_tmp_path() {
        let p = String::from("./test/test/test");
        let mut t2 = std::env::current_dir().unwrap();
        t2.push("test");

        let subject = UnityPackage::new("test.unitypackage", None, Some(p.clone())).unwrap();

        assert_eq!(subject.get_tmp_dir().unwrap(), PathBuf::from(p));
        assert_eq!(subject.get_target_dir().unwrap(), t2);
    }

    #[test]
    fn test_new_function_with_target_path() {
        let path = std::env::current_dir().unwrap();
        let mut origin = path.clone();
        origin.push("origin/file.unitypackage");

        let mut target = path.clone();
        target.push("target");

        let t = target.clone().into_os_string().into_string().unwrap();
        let o = origin.clone().into_os_string().into_string().unwrap();

        let subject = UnityPackage::new(&o, Some(t), None).unwrap();

        assert_eq!(subject.get_target_dir().unwrap(), target);
        assert_eq!(subject.get_package_file_name().unwrap(), "file");
        assert_eq!(
            subject.get_path(),
            origin.into_os_string().into_string().unwrap()
        );
    }

    #[test]
    #[serial]
    fn test_asset_file_internals() {
        let base = get_test_base_path();
        println!("{:?}", base);
        let mut tmp = base.clone();
        tmp.push("assets/tmp");

        let mut target = base.clone();
        target.push("assets/target");

        let mut absolute_path = base.clone();
        absolute_path.push("assets/test.unitypackage");

        let mut subject = match UnityPackage::new(
            absolute_path.to_str().unwrap(),
            Some(target.to_str().unwrap().to_string()),
            Some(tmp.to_str().unwrap().to_string()),
        ) {
            Ok(s) => s,
            Err(_) => panic!("Could not unpack package"),
        };

        match subject.unpack_package(true) {
            Ok(e) => e,
            Err(e) => {
                panic!("{}", e)
            }
        };

        let file = match subject.get_file(&"1af567ac160bb164fb19b8cb9b55b34b".to_string()) {
            Some(f) => f,
            None => {
                panic!("The file does not exist in this package.")
            }
        };

        let working_dir = get_test_base_path();

        let mut absolute_meta = working_dir.clone();
        absolute_meta.push("assets/Assets/Textures/Ground/IMGP1287.jpg.unitymeta");

        let mut absolute_target = working_dir.clone();
        absolute_target.push("assets/Assets/Textures/Ground/IMGP1287.jpg");

        assert_eq!(file.get_guid(), "1af567ac160bb164fb19b8cb9b55b34b");

        assert_eq!(
            file.get_relative_asset_path().to_str().unwrap(),
            "Assets/Textures/Ground/IMGP1287.jpg"
        );

        std::fs::remove_dir_all(target.clone()).unwrap();

        assert!(!target.exists());
        assert!(!tmp.exists());
    }

    // Check if tmp directory still exists after unpacking.
    #[test]
    #[serial]
    fn test_deleting_tmp_files() {
        let base = get_test_base_path();
        println!("{:?}", base);
        let mut tmp = base.clone();
        tmp.push("assets/tmp");

        let mut target = base.clone();
        target.push("assets/target");

        let mut absolute_path = base.clone();
        absolute_path.push("assets/test.unitypackage");

        let mut subject = UnityPackage::new(
            absolute_path.to_str().unwrap(),
            Some(target.to_str().unwrap().to_string()),
            Some(tmp.to_str().unwrap().to_string()),
        )
        .unwrap();

        subject.unpack_package(false).unwrap();

        assert!(tmp.exists());

        // Clean up
        std::fs::remove_dir_all(target.clone()).unwrap();
        std::fs::remove_dir_all(tmp.clone()).unwrap();

        assert!(!target.exists());
        assert!(!tmp.exists());
    }
}
