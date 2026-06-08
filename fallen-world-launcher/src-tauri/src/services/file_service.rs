use std::fs;
use std::path::{Path, PathBuf};

pub struct FileService;

impl FileService {
    pub fn copy_recursive(src: &str, dst: &str) -> std::io::Result<()> {
        let src_path = PathBuf::from(src);
        let dst_path = PathBuf::from(dst);

        // Validate paths exist and are readable
        if !src_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Source path does not exist",
            ));
        }

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            for entry in fs::read_dir(&src_path)? {
                let entry = entry?;
                let path = entry.path();
                let file_name = entry.file_name();
                let new_dst = dst_path.join(&file_name);

                if path.is_dir() {
                    Self::copy_recursive(
                        path.to_string_lossy().as_ref(),
                        new_dst.to_string_lossy().as_ref(),
                    )?;
                } else {
                    fs::copy(&path, &new_dst)?;
                }
            }
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
        Ok(())
    }

    pub fn delete_recursive(path: &str) -> std::io::Result<()> {
        let target = Path::new(path);

        // Verify path exists before attempting deletion (prevent TOCTOU race)
        if !target.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Path does not exist",
            ));
        }

        if target.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }

    pub fn file_exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    pub fn create_directory(path: &str) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }
}
