use crate::prelude::UnityPackageReaderError;
use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone)]
pub struct UnityAssetFile {
    /// The guid of this asset. This equals
    /// the directory of the asset in the tmp directory.
    guid: String,
    /// Absolute path to the asset.
    asset: PathBuf,
    /// Relative path inside the target folder.
    target: PathBuf,
    /// Absolute path to the meta data file
    meta: PathBuf,
    /// True, if an asset is a folder (which means, there is none)
    is_folder: bool,
}

impl UnityAssetFile {
    pub fn get_guid(&self) -> &String {
        &self.guid
    }
    pub fn get_absolute_asset_path(&self) -> &PathBuf {
        &self.asset
    }
    pub fn get_relative_asset_path(&self) -> &PathBuf {
        &self.target
    }
    pub fn get_absolute_meta_file_path(&self) -> &PathBuf {
        &self.meta
    }
    pub fn is_folder(&self) -> bool {
        self.is_folder
    }

    pub fn from(path: PathBuf) -> Result<Self, UnityPackageReaderError> {
        let h = match path.file_name() {
            Some(h) => h.to_str(),
            None => {
                return Err(UnityPackageReaderError::PathError);
            }
        };

        let hash = match h {
            Some(e) => String::from(e),
            None => {
                return Err(UnityPackageReaderError::PathError);
            }
        };

        let origin = path.clone();
        let mut asset = path.clone();
        asset.push("asset");

        let mut pathname = origin.clone();
        pathname.push("pathname");

        let mut meta = path.clone();
        meta.push("asset.meta");

        let target = match Self::get_relative_path(&pathname) {
            Ok(e) => e,
            Err(_) => {
                return Err(UnityPackageReaderError::CorruptPackage);
            }
        };

        let is_folder = match Self::get_is_folder(&meta) {
            Ok(e) => e,
            Err(_) => {
                return Err(UnityPackageReaderError::CouldReadMetaFile);
            }
        };

        Ok(UnityAssetFile {
            guid: hash,
            asset,
            target,
            meta,
            is_folder,
        })
    }

    fn get_relative_path(file: &PathBuf) -> Result<PathBuf, UnityPackageReaderError> {
        let content = match fs::read_to_string(file) {
            Ok(e) => e,
            Err(_) => {
                return Err(UnityPackageReaderError::CorruptPackage);
            }
        };

        Ok(PathBuf::from(content))
    }

    fn get_is_folder(file: &PathBuf) -> Result<bool, UnityPackageReaderError> {
        let content = match fs::read_to_string(file) {
            Ok(e) => e,
            Err(_) => {
                return Err(UnityPackageReaderError::CorruptPackage);
            }
        };

        Ok(content.contains("folderAsset: yes"))
    }

    /// Copy this file from the tmp folder to the target folder. The folder structure
    /// inside the unitypackage file will be maintained. So this method creates all
    /// directories inside the target folder, that are needed to achive this.
    /// Besides the asset itself the meta file is copied over as well. However its
    /// extension is changed to .unitymeta to destinguish it from other meta files.
    pub fn copy_asset(&mut self, target_path: &Path) -> Result<(), UnityPackageReaderError> {
        if self.is_folder() {
            return Ok(());
        }

        let mut absolute_target_path = target_path.to_path_buf();
        // add the path we extracted from to the target directory.
        absolute_target_path.push(&self.target);
        let parent = match absolute_target_path.parent() {
            Some(e) => e.to_path_buf(),
            None => {
                return Err(UnityPackageReaderError::TargetDirectoryCouldNotBeCreated);
            }
        };

        if !parent.as_path().exists() {
            match std::fs::create_dir_all(parent) {
                Ok(_) => {}
                Err(_) => {
                    return Err(UnityPackageReaderError::TargetDirectoryCouldNotBeCreated);
                }
            }
        }

        // println!("Absolute target path: {:?}", absolute_target_path);
        let asset = match std::fs::rename(&self.asset, absolute_target_path.clone()) {
            Ok(_) => absolute_target_path,
            Err(e) => {
                println!("{:?}", e);
                return Err(UnityPackageReaderError::CorruptPackage);
            }
        };

        let mut meta_target_file_name = asset.to_path_buf();
        let f = match meta_target_file_name.file_name()
        {
            Some(s) => s.to_str(),
            None => { return Err(UnityPackageReaderError::CorruptPackage); }
        };

        let mut file_name = match f {
            Some(s) => String::from(s),
            None => { return Err(UnityPackageReaderError::CorruptPackage); }
        };

        file_name.push_str(".unitymeta");
        meta_target_file_name = match meta_target_file_name.parent() {
            Some(s) => s.to_path_buf(),
            None => { return Err(UnityPackageReaderError::CorruptPackage); }
        };

        meta_target_file_name.push(file_name);
        match std::fs::rename(&self.meta, meta_target_file_name) {
            Ok(_) => {}
            Err(_) => {
                return Err(UnityPackageReaderError::CorruptPackage);
            }
        };

        Ok(())
    }
}
