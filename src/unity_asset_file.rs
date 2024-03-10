use std::{fs, path::PathBuf};

use crate::prelude::UnityPackageReaderError;

#[derive(Debug, Clone)]
pub struct UnityAssetFile {
    /// The guid of this asset. This equals
    /// the directory of the asset in the tmp directory.
    guid : String,
    /// Absolute path to the asset.
    asset : PathBuf,
    /// Relative path inside the target folder.
    target : PathBuf,
    /// Absolute path to the meta data file 
    meta : PathBuf,
}

impl UnityAssetFile {
    pub fn get_guid(&self) -> &String { &self.guid }
    pub fn get_absolute_asset_path(&self) -> &PathBuf { &self.asset }
    pub fn get_relative_asset_path(&self) -> &PathBuf { &self.target }
    pub fn get_absolute_meta_file_path(&self) -> &PathBuf { &self.meta }

    pub fn from(path : PathBuf) -> Result<Self, UnityPackageReaderError> {
        let h = match path.file_name() {
            Some(h) => { h.to_str() },
            None => { return Err(UnityPackageReaderError::PathError); },
        };

        let hash = match h {
            Some(e) => { String::from(e) },
            None => { return Err(UnityPackageReaderError::PathError); },
        };

        let origin = path.clone();
        let mut asset = path.clone();
        asset.push("asset");

        let mut pathname = origin.clone();
        pathname.push("pathname");

        let mut meta = path.clone();
        meta.push("asset.meta");

        let target = match Self::get_relative_path(&pathname) {
            Ok(e) => { e },
            Err(_) => { return Err(UnityPackageReaderError::CorruptPackage); },
        };

        Ok(UnityAssetFile {
            guid: hash,
            asset, 
            target,
            meta
        })
    }

    fn get_relative_path(file : &PathBuf) -> Result<PathBuf, UnityPackageReaderError> {
        let content = match fs::read_to_string(file) {
            Ok(e) => { e },
            Err(_) => { return Err(UnityPackageReaderError::CorruptPackage); },
        };

        Ok(PathBuf::from(content))
    } 

    /// Copy this file from the tmp folder to the target folder. The folder structure
    /// inside the unitypackage file will be maintained. So this method creates all
    /// directories inside the target folder, that are needed to achive this.
    /// Besides the asset itself the meta file is copied over as well. However its
    /// extension is changed to .unitymeta to destinguish it from other meta files.
    pub fn copy_asset(&self, target_path : PathBuf) -> Result<PathBuf, UnityPackageReaderError> {
        let mut absolute_target_path = target_path.clone();
        // add the path we extracted from to the target directory.
        absolute_target_path.push(&self.target);
        
        // println!("Absolute target path: {:?}", absolute_target_path);
        let asset = match std::fs::rename(&self.asset, absolute_target_path.clone()) {
            Ok(_) => { absolute_target_path },
            Err(_) => { return Err(UnityPackageReaderError::CorruptPackage); },
        };

        let mut meta_target_path = target_path.clone();

        let binding = asset.clone();
        let target_file_name_tmp = match binding.file_stem() {
            Some(stem) => {  stem.to_str()},
            None => { return Err(UnityPackageReaderError::CorruptPackage);},
        };

        let mut meta_target_file_name = match target_file_name_tmp {
            Some(stem) => { String::from(stem) },
            None => { return Err(UnityPackageReaderError::CorruptPackage); },
        };

        meta_target_file_name.push_str(".unitymeta");
        meta_target_path.push(meta_target_file_name.as_str());
        match std::fs::rename(&self.meta, meta_target_path) {
            Ok(_) => { },
            Err(_) => { return Err(UnityPackageReaderError::CorruptPackage); },
        };

        Ok(asset)
    }
}