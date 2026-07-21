use crate::storage::{read_bytes, write_bytes};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

fn secret_path() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "shijuan", "shenqi")
        .ok_or_else(|| "无法定位配置目录".to_string())?;
    let dir = dirs.config_dir();
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    Ok(dir.join("api-key.bin"))
}

pub fn save_api_key(api_key: &str) -> Result<(), String> {
    let encrypted = protect(api_key.as_bytes())?;
    write_bytes(&secret_path()?, &encrypted)
}

pub fn load_api_key() -> Result<Option<String>, String> {
    let Some(encrypted) = read_bytes(&secret_path()?)? else {
        return Ok(None);
    };
    let plain = unprotect(&encrypted)?;
    String::from_utf8(plain)
        .map(Some)
        .map_err(|_| "API Key 解密后不是有效 UTF-8".to_string())
}

pub fn clear_api_key() -> Result<(), String> {
    let path = secret_path()?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn protect(input: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let result = unsafe {
        CryptProtectData(
            &mut input_blob,
            null(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output_blob,
        )
    };
    if result == 0 {
        return Err(format!(
            "API Key 加密失败: {}",
            std::io::Error::last_os_error()
        ));
    }
    let bytes = unsafe {
        std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
    };
    unsafe { LocalFree(output_blob.pbData as *mut core::ffi::c_void) };
    Ok(bytes)
}

#[cfg(windows)]
fn unprotect(input: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let result = unsafe {
        CryptUnprotectData(
            &mut input_blob,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output_blob,
        )
    };
    if result == 0 {
        return Err(format!(
            "API Key 解密失败: {}",
            std::io::Error::last_os_error()
        ));
    }
    let bytes = unsafe {
        std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
    };
    unsafe { LocalFree(output_blob.pbData as *mut core::ffi::c_void) };
    Ok(bytes)
}

#[cfg(not(windows))]
fn protect(_input: &[u8]) -> Result<Vec<u8>, String> {
    Err("当前平台尚未配置系统凭据加密".into())
}

#[cfg(not(windows))]
fn unprotect(_input: &[u8]) -> Result<Vec<u8>, String> {
    Err("当前平台尚未配置系统凭据解密".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn dpapi_round_trip() {
        let plain = b"test-api-key-not-a-real-secret";
        let encrypted = protect(plain).unwrap();
        assert_ne!(encrypted, plain);
        assert_eq!(unprotect(&encrypted).unwrap(), plain);
    }
}
