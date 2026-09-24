//! Zip compression helper for folders and single files.
//! Excludes build artifacts and VCS directories (.git, bin, obj, node_modules).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::error::{Error, Result};

const EXCLUDE_NAMES: &[&str] = &[".git", "bin", "obj", "node_modules", ".DS_Store", "target"];

pub fn zip_single_file(file_path: &Path) -> Result<(PathBuf, String)> {
    let file_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::invalid(format!("Invalid file path: {}", file_path.display())))?;

    let stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or(file_name);
    let blob_name = format!("{stem}.zip");

    let temp_zip_path = temp_zip_path(&blob_name);
    let out_file = File::create(&temp_zip_path)?;
    let mut zip = ZipWriter::new(out_file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(file_name, options).map_err(|e| Error::other(e.to_string()))?;
    let mut input = File::open(file_path)?;
    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;
    zip.write_all(&buffer)?;
    zip.finish().map_err(|e| Error::other(e.to_string()))?;

    Ok((temp_zip_path, blob_name))
}

pub fn zip_directory(source_dir: &Path) -> Result<(PathBuf, String)> {
    let dir_name = source_dir.file_name().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).unwrap_or("archive");
    let blob_name = format!("{dir_name}.zip");

    let temp_zip_path = temp_zip_path(&blob_name);
    let out_file = File::create(&temp_zip_path)?;
    let mut zip = ZipWriter::new(out_file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    add_dir_to_zip(&mut zip, source_dir, source_dir, options)?;
    zip.finish().map_err(|e| Error::other(e.to_string()))?;

    Ok((temp_zip_path, blob_name))
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    base_dir: &Path,
    current_dir: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if EXCLUDE_NAMES.iter().any(|exc| exc.eq_ignore_ascii_case(&name_str)) {
            continue;
        }

        let rel_path =
            path.strip_prefix(base_dir).map_err(|e| Error::other(e.to_string()))?.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            let dir_entry = if rel_path.ends_with('/') { rel_path.to_string() } else { format!("{rel_path}/") };
            zip.add_directory(&dir_entry, options).map_err(|e| Error::other(e.to_string()))?;
            add_dir_to_zip(zip, base_dir, &path, options)?;
        } else if path.is_file() {
            zip.start_file(&rel_path, options).map_err(|e| Error::other(e.to_string()))?;
            let mut file = File::open(&path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
    }
    Ok(())
}

fn temp_zip_path(blob_name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    std::env::temp_dir().join(format!("storage-cli-{nonce}-{blob_name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_single_file() {
        let temp_dir = std::env::temp_dir().join("storage_zip_test_file");
        let _ = fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("test.txt");
        fs::write(&test_file, b"hello world").unwrap();

        let (zip_path, blob_name) = zip_single_file(&test_file).unwrap();
        assert_eq!(blob_name, "test.zip");
        assert!(zip_path.exists());
        let _ = fs::remove_file(zip_path);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_zip_directory() {
        let temp_dir = std::env::temp_dir().join("storage_zip_test_dir");
        let _ = fs::create_dir_all(temp_dir.join("subdir"));
        let _ = fs::create_dir_all(temp_dir.join("node_modules"));
        fs::write(temp_dir.join("file1.txt"), b"file 1 content").unwrap();
        fs::write(temp_dir.join("subdir/file2.txt"), b"file 2 content").unwrap();
        fs::write(temp_dir.join("node_modules/excluded.txt"), b"should not be in zip").unwrap();

        let (zip_path, blob_name) = zip_directory(&temp_dir).unwrap();
        assert!(blob_name.ends_with(".zip"));
        assert!(zip_path.exists());

        // Read zip back and inspect entries
        let file = File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len()).map(|i| archive.by_index(i).unwrap().name().to_string()).collect();

        assert!(names.iter().any(|n| n == "file1.txt"));
        assert!(names.iter().any(|n| n == "subdir/file2.txt"));
        assert!(!names.iter().any(|n| n.contains("node_modules")));

        let _ = fs::remove_file(zip_path);
        let _ = fs::remove_dir_all(temp_dir);
    }
}
