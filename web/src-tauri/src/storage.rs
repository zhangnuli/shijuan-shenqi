use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn backup_path(path: &Path) -> PathBuf {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("data");
    path.with_extension(format!("{extension}.bak"))
}

fn temporary_path(path: &Path) -> PathBuf {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("data");
    let suffix = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("{extension}.tmp-{}-{suffix}", std::process::id()))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let backup = backup_path(path);
    let mut first_error = None;
    for candidate in [path, backup.as_path()] {
        if !candidate.exists() {
            continue;
        }
        match fs::read_to_string(candidate)
            .map_err(|e| e.to_string())
            .and_then(|content| serde_json::from_str(&content).map_err(|e| e.to_string()))
        {
            Ok(value) => return Ok(Some(value)),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }
    match first_error {
        Some(error) => Err(format!("数据文件及备份均无法读取: {error}")),
        None => Ok(None),
    }
}

pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    write_bytes(path, &bytes)
}

pub fn read_bytes(path: &Path) -> Result<Option<Vec<u8>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    fs::read(path).map(Some).map_err(|e| e.to_string())
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = temporary_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);

    if path.exists() {
        let backup = backup_path(path);
        fs::copy(path, &backup).map_err(|e| e.to_string())?;
        if let Ok(backup_file) = File::open(&backup) {
            let _ = backup_file.sync_all();
        }
    }
    replace_file(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(|e| e.to_string())
}

pub fn unique_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{nanos:x}-{counter:x}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn writes_and_recovers_json() {
        let dir = std::env::temp_dir().join(unique_id("shijuan-storage-test"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data.json");
        write_json(&path, &json!({"value": 1})).unwrap();
        write_json(&path, &json!({"value": 2})).unwrap();
        fs::write(&path, "broken").unwrap();
        let recovered: serde_json::Value = read_json(&path).unwrap().unwrap();
        assert_eq!(recovered["value"], 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn generated_ids_do_not_collide() {
        assert_ne!(unique_id("item"), unique_id("item"));
    }
}
