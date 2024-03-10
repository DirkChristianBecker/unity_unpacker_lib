use flate2::read::GzDecoder;
use rust_tools::prelude::*;
use std::{
    collections::HashMap, fmt, fs, path::{Path, PathBuf}
};
use tar::Archive;

use crate::prelude::UnityAssetFile;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum UnityPackageReaderError {
    PackageNotFound,
    CorruptPackage,
    DirectoryCouldNotBeCreated,
    WorkingDirectoryError,
    PathError,
    NotAPackageFile,
}

impl fmt::Display for UnityPackageReaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnityPackageReaderError::PackageNotFound => write!(f, "File not found"),
            UnityPackageReaderError::PathError => write!(f, "Could not create a valid path."),
            UnityPackageReaderError::CorruptPackage => write!(f, "Could not unpack a unity package, it seems to be corrupt."),
            UnityPackageReaderError::DirectoryCouldNotBeCreated => write!(f, "Could not create the temporary directory to extract files to."),
            UnityPackageReaderError::WorkingDirectoryError => write!(f, "Could not determine the current working directory. Consider passing an absolute path to the file to create a UnityPackage."),
            UnityPackageReaderError::NotAPackageFile => write!(f, "The given path seems to point to a directory."),
        }
    }
}

pub struct UnityPackage {
    /// The name of the file to unpack. 
    path: String,
    /// The target directory. If none is set the current working directory and the name of the package will be used
    target_path : Option<String>,
    /// We have to unpack the file into a tmp directory
    temp_directory : Option<String>,
    /// The files we found hashed by the guid
    files : HashMap<String, UnityAssetFile>,
}

impl UnityPackage {
    /// Creates a new UnityPackage. The given file name is either the absolute path
    /// to the package on disk or the name of the file in the current
    /// working directory (or a subdirectory of the current working directory).
    pub fn new(
        file_name: &str, 
        target_path : Option<String>, 
        temp_directory : Option<String>) -> Result<Self, UnityPackageReaderError> {
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

        Ok(UnityPackage { 
            path, 
            target_path, 
            temp_directory,
            files : HashMap::new(),
         })
    }
    
    pub fn get_path(&self) -> String { self.path.clone() }
    pub fn get_file(&self, guid : &String) -> Option<&UnityAssetFile> { self.files.get(guid) }

    /// The default tmp directory is always the current [working directory]/tmp
    fn get_default_tmp_dir(&self) -> Result<PathBuf, UnityPackageReaderError> {
        match &self.temp_directory {
            Some(s) => {
                Ok(PathBuf::from(s))
            },
            None => {
                if let Ok(mut working_dir) = std::env::current_dir() {
                    working_dir.push("tmp");
                    Ok(working_dir)            
                } else {
                    Err(UnityPackageReaderError::WorkingDirectoryError)
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
                    Err(UnityPackageReaderError::NotAPackageFile)
                }
                
            },
            None => {
                Err(UnityPackageReaderError::NotAPackageFile)
            },
        }
    }

    /// Get the target directory. If the target has been set by the user
    /// then this directory is beeing return. 
    /// Otherwise we use the current working directory and append the file name
    /// of the package. 
    fn get_default_target_dir(&self) -> Result<PathBuf, UnityPackageReaderError> {
        match &self.target_path {
            Some(s) => {
                Ok(PathBuf::from(s))
            },

            None => {
                match self.get_package_file_name() {
                    Ok(s) => {
                        match std::env::current_dir() {
                            Ok(mut r) => {
                                r.push(s);
                                Ok(r)
                            },
                            Err(_) => {
                                Err(UnityPackageReaderError::WorkingDirectoryError)
                            },
                        }                        
                    },
                    Err(_) => 
                    {
                        Err(UnityPackageReaderError::NotAPackageFile)
                    },
                }
            }
        }
    }

    pub fn unpack_package(
        &mut self,
        extract_to: Option<&Path>,
    ) -> Result<PathBuf, UnityPackageReaderError> {
        println!("{}", self.path);

        let tmp = get_file_as_byte_vec(Path::new(self.path.clone().as_str()));
        match tmp {
            Ok(bytes) => {
                let path = match extract_to {
                    Some(t) => PathBuf::from(t),
                    None => match self.get_default_tmp_dir() {
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
                    Ok(_) => 
                    {
                        let r = path;
                        match self.copy_files_to_target() {
                            Ok(_) => { Ok(r) },
                            Err(e) => { Err(e) } ,
                        }
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

    fn copy_files_to_target(&mut self) -> Result<usize, UnityPackageReaderError> {        
        let p = self.get_default_tmp_dir();
        let t = self.get_default_target_dir();

        let _target = match t {
            Ok(f) => { f },
            Err(e) => return Err(e),
        };

        let origin = match p {
            Ok(f) => { f },
            Err(e) => return Err(e),
        };

        let files = match fs::read_dir(origin) {
            Ok(f) => { f },
            Err(_) => {  return Err(UnityPackageReaderError::DirectoryCouldNotBeCreated); },
        };

        for entry in files {
            let entry = match entry {
                Ok(f) => { f },
                Err(_) => return Err(UnityPackageReaderError::CorruptPackage),
            };

            let p = entry.path();
            let asset_file = UnityAssetFile::from(p);
            match asset_file {
                Ok(a) => {
                    self.files.insert(a.get_guid().clone(), a);
                },
                Err(_) => {
                    return Err(UnityPackageReaderError::CorruptPackage);
                },
            }
        }

        Ok(self.files.len())

    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tmp_dir() {
        let mut p = std::env::current_dir().unwrap();
        p.push("tmp");

        let mut t2 = std::env::current_dir().unwrap();
        t2.push("file");

        let item = UnityPackage::new("file.unitypackage", None, None).unwrap();

        assert_eq!(p, item.get_default_tmp_dir().unwrap());
        assert_eq!(item.get_default_target_dir().unwrap(), t2);
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
        assert_eq!(package.get_default_target_dir().unwrap(), t2);
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

        let subject =
            UnityPackage::new(p.clone().into_os_string().into_string().unwrap().as_str(), None, None).unwrap();
        assert_eq!(p.into_os_string().into_string().unwrap(), subject.path);
        assert_eq!(subject.get_default_target_dir().unwrap(), t2);
    }

    #[test]
    fn test_new_function_with_tmp_path() {
        let p = String::from("./test/test/test");
        let mut t2 = std::env::current_dir().unwrap();
        t2.push("test");

        let subject = UnityPackage::new("test.unitypackage", None, Some(p.clone())).unwrap();

        assert_eq!(subject.get_default_tmp_dir().unwrap(), PathBuf::from(p));
        assert_eq!(subject.get_default_target_dir().unwrap(), t2);
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

        assert_eq!(subject.get_default_target_dir().unwrap(), target);
        assert_eq!(subject.get_package_file_name().unwrap(), "file");
        assert_eq!(subject.get_path(), origin.into_os_string().into_string().unwrap());
    }

    #[test]
    fn test_asset_file_internals() {
        let mut subject = match UnityPackage::new("assets/test.unitypackage", None, None) {
            Ok(s) => { s },
            Err(_) => panic!("Could not unpack package"),
        };
        
        let absolute_asset_target_path = match subject.unpack_package(None) {
            Ok(e) => { e },
            Err(e) => { panic!("{}", e) },
        };

        let file = match subject.get_file(&"0b0b00b564994434b8c9fc09a95a4acc".to_string()) {
            Some(f) => { f },
            None => { panic!("The file does not exist in this package.")},
        };

        let working_dir = match std::env::current_dir() {
            Ok(e) => { e },
            Err(_) => { panic!("Could not lookup working directory.")},
        };

        let mut absolute_meta = working_dir.clone();
        absolute_meta.push("test/Assets/Synty/InterfaceApocalypseHUD/Sprites/HUD/SPR_HUD_Frame_Border_Grunge_Med_Simple.png.unitymeta");

        let mut absolute_target = working_dir.clone();
        absolute_target.push("test/Assets/Synty/InterfaceApocalypseHUD/Sprites/HUD/SPR_HUD_Frame_Border_Grunge_Med_Simple.png");

        assert_eq!(file.get_guid(), "0b0b00b564994434b8c9fc09a95a4acc");
        assert_eq!(
            file.get_relative_asset_path().to_str().unwrap(), 
            "Assets/Synty/InterfaceApocalypseHUD/Sprites/HUD/SPR_HUD_Frame_Border_Grunge_Med_Simple.png");
        assert_eq!(
            absolute_asset_target_path, 
            absolute_target);
    }
}