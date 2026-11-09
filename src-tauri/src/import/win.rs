//! Operaciones de archivo en Windows. El temporal se crea con `CREATE_NEW`.
//! Publicar usa `MoveFileExW` sin `MOVEFILE_REPLACE_EXISTING`.
//! El original de un movimiento se borra con el mismo manejador que se ha comprobado.

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::Globalization::{NormalizationC, NormalizeString};
use windows::Win32::Security::Cryptography::{
    BCRYPT_ALG_HANDLE, BCRYPT_HASH_HANDLE, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS,
    BCRYPT_SHA256_ALGORITHM, BCryptCloseAlgorithmProvider, BCryptCreateHash, BCryptDestroyHash,
    BCryptFinishHash, BCryptHashData, BCryptOpenAlgorithmProvider,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, DELETE, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_BASIC_INFO,
    FILE_BEGIN, FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
    FILE_DISPOSITION_INFO_EX, FILE_DISPOSITION_INFO_EX_FLAGS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_GENERIC_READ, FILE_ID_INFO, FILE_NAME_NORMALIZED, FILE_SHARE_READ, FileBasicInfo,
    FileDispositionInfoEx, FileIdInfo, GetDiskFreeSpaceExW, GetFileInformationByHandleEx,
    GetFileSizeEx, GetFileTime, GetFinalPathNameByHandleW, GetFullPathNameW,
    MOVEFILE_WRITE_THROUGH, MoveFileExW, OPEN_EXISTING, ReadFile, SetFileInformationByHandle,
    SetFilePointerEx,
};
use windows::Win32::UI::WindowsAndMessaging::CharLowerBuffW;
use windows::core::PCWSTR;

use crate::storage::is_reparse_point;

pub const COPY_CHUNK: usize = 1024 * 1024;

#[derive(Debug)]
pub enum PublishError {
    Occupied,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct FileFacts {
    pub size: u64,
    pub modified: String,
    pub volume_serial: u64,
    pub file_id: [u8; 16],
}

pub struct OpenedFile {
    handle: HANDLE,
    pub facts: FileFacts,
}

impl Drop for OpenedFile {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

pub fn path_key(path: &Path) -> Result<String, String> {
    let mut full = full_path(path)?;
    nfc(&mut full)?;
    unsafe {
        CharLowerBuffW(&mut full);
    }
    let text = OsString::from_wide(&full).to_string_lossy().into_owned();
    let without_prefix = text
        .strip_prefix(r"\\?\")
        .unwrap_or(text.as_str())
        .replace('/', "\\");
    Ok(without_prefix.replace('\\', "/"))
}

pub fn is_inside(root: &Path, child: &Path) -> Result<bool, String> {
    let root_key = path_key(root)?;
    let child_key = path_key(child)?;
    Ok(child_key.starts_with(&(root_key + "/")))
}

pub fn free_bytes(directory: &Path) -> Result<u64, String> {
    let wide = wide_path(directory);
    let mut free = 0u64;
    unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut free as *mut u64),
            None,
            None,
        )
        .map_err(|error| format!("No se ha podido consultar el espacio libre: {error}"))?;
    }
    Ok(free)
}

pub fn open_source(path: &Path) -> Result<OpenedFile, String> {
    if is_reparse_point(path) {
        return Err("El origen es un enlace y no se abre.".into());
    }
    let wide = wide_path(path);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0 | DELETE.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
        .map_err(|error| format!("No se ha podido abrir el origen: {error}"))?
    };
    let opened = OpenedFile {
        handle,
        facts: read_facts(handle)?,
    };
    if opened.facts.size > i64::MAX as u64 {
        return Err("El archivo es demasiado grande.".into());
    }
    ensure_regular_handle(handle)?;
    let final_path = final_path(handle)?;
    if path_key(path)? != path_key(&final_path)? {
        return Err("La ruta del origen cambia al abrirlo y no se importa.".into());
    }
    Ok(opened)
}

pub fn copy_handle_to_new_file(
    source: &OpenedFile,
    destination: &Path,
    chunk: usize,
    interrupt_after: Option<u64>,
    cancel: &std::sync::atomic::AtomicBool,
    mut on_chunk: impl FnMut(),
) -> Result<(), String> {
    if destination.exists() {
        return Err("El temporal ya existe y no se sobrescribe.".into());
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| format!("No se ha podido crear el temporal: {error}"))?;
    let mut written = 0u64;
    let mut buffer = vec![0u8; chunk.max(1)];
    rewind(source.handle)?;
    loop {
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            drop(file);
            return Err("cancelada".into());
        }
        let mut read = 0u32;
        unsafe {
            ReadFile(source.handle, Some(&mut buffer), Some(&mut read), None)
                .map_err(|error| format!("No se ha podido leer el origen: {error}"))?;
        }
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut &file, &buffer[..read as usize])
            .map_err(|error| format!("No se ha podido escribir el temporal: {error}"))?;
        written += u64::from(read);
        file.sync_data()
            .map_err(|error| format!("No se ha podido confirmar el temporal: {error}"))?;
        on_chunk();
        if interrupt_after.is_some_and(|limit| written >= limit) {
            drop(file);
            return Err("interrumpida".into());
        }
    }
    file.sync_all()
        .map_err(|error| format!("No se ha podido cerrar el temporal: {error}"))?;
    if written != source.facts.size {
        return Err("El origen ha cambiado durante la copia.".into());
    }
    let current = read_facts(source.handle)?;
    if current.size != source.facts.size
        || current.modified != source.facts.modified
        || current.file_id != source.facts.file_id
        || current.volume_serial != source.facts.volume_serial
    {
        return Err("El origen ha cambiado durante la copia.".into());
    }
    Ok(())
}

/// Publica un archivo que esta operación ha creado. No sustituye un destino existente.
pub fn publish_exclusive(from: &Path, to: &Path) -> Result<(), PublishError> {
    if to.exists() {
        return Err(PublishError::Occupied);
    }
    let source = wide_path(from);
    let destination = wide_path(to);
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| {
            let code = error.code().0 as u32 & 0xFFFF;
            if code == 183 || code == 80 || to.exists() {
                PublishError::Occupied
            } else {
                PublishError::Failed(format!("No se ha podido publicar el archivo: {error}"))
            }
        })
    }
}

pub fn hash_handle(file: &OpenedFile) -> Result<String, String> {
    rewind(file.handle)?;
    hash_reader(|buffer| {
        let mut read = 0u32;
        unsafe {
            ReadFile(file.handle, Some(buffer), Some(&mut read), None)
                .map_err(|error| format!("No se ha podido leer para la huella: {error}"))?;
        }
        Ok(read as usize)
    })
}

pub fn hash_path(path: &Path) -> Result<String, String> {
    let wide = wide_path(path);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
        .map_err(|error| format!("No se ha podido abrir el destino para la huella: {error}"))?
    };
    let opened = OpenedFile {
        handle,
        facts: read_facts(handle)?,
    };
    if is_reparse_handle(opened.handle)? {
        return Err("El destino es un enlace y no se comprueba.".into());
    }
    hash_handle(&opened)
}

/// Borra el archivo del manejador ya abierto, no la ruta. El llamador tiene que haber
/// comparado antes la huella de este mismo manejador.
pub fn delete_open_file(file: &OpenedFile) -> Result<(), String> {
    let info = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_INFO_EX_FLAGS(
            FILE_DISPOSITION_FLAG_DELETE.0 | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS.0,
        ),
    };
    unsafe {
        SetFileInformationByHandle(
            file.handle,
            FileDispositionInfoEx,
            &info as *const _ as *const _,
            std::mem::size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
        )
        .map_err(|error| {
            format!("No se ha podido eliminar el original por su identificador abierto: {error}")
        })?;
    }
    Ok(())
}

pub fn current_facts(file: &OpenedFile) -> Result<FileFacts, String> {
    read_facts(file.handle)
}

pub fn destination_facts(path: &Path) -> Result<FileFacts, String> {
    if is_reparse_point(path) {
        return Err("El destino es un enlace.".into());
    }
    let wide = wide_path(path);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
        .map_err(|error| format!("No se ha podido abrir el destino: {error}"))?
    };
    let opened = OpenedFile {
        handle,
        facts: read_facts(handle)?,
    };
    ensure_regular_handle(opened.handle)?;
    Ok(opened.facts.clone())
}

pub fn file_id_of(facts: &FileFacts) -> Option<String> {
    if facts.file_id.iter().all(|byte| *byte == 0) && facts.volume_serial == 0 {
        None
    } else {
        Some(hex(&facts.file_id))
    }
}

fn hash_reader(mut read: impl FnMut(&mut [u8]) -> Result<usize, String>) -> Result<String, String> {
    let mut algorithm = BCRYPT_ALG_HANDLE::default();
    let mut hash = BCRYPT_HASH_HANDLE::default();
    unsafe {
        let status = BCryptOpenAlgorithmProvider(
            &mut algorithm,
            BCRYPT_SHA256_ALGORITHM,
            PCWSTR(std::ptr::null()),
            BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
        );
        if status.0 < 0 {
            return Err("No se ha podido preparar SHA-256.".into());
        }
        let status = BCryptCreateHash(algorithm, &mut hash, None, None, 0);
        if status.0 < 0 {
            let _ = BCryptCloseAlgorithmProvider(algorithm, 0);
            return Err("No se ha podido preparar SHA-256.".into());
        }
    }
    let mut buffer = vec![0u8; COPY_CHUNK];
    let result = (|| {
        loop {
            let read = read(&mut buffer)?;
            if read == 0 {
                break;
            }
            unsafe {
                let status = BCryptHashData(hash, &buffer[..read], 0);
                if status.0 < 0 {
                    return Err("No se ha podido calcular SHA-256.".into());
                }
            }
        }
        let mut output = [0u8; 32];
        unsafe {
            let status = BCryptFinishHash(hash, &mut output, 0);
            if status.0 < 0 {
                return Err("No se ha podido calcular SHA-256.".into());
            }
        }
        Ok(format!("sha256:{}", hex(&output)))
    })();
    unsafe {
        let _ = BCryptDestroyHash(hash);
        let _ = BCryptCloseAlgorithmProvider(algorithm, 0);
    }
    result
}

fn read_facts(handle: HANDLE) -> Result<FileFacts, String> {
    let mut size = 0i64;
    unsafe {
        GetFileSizeEx(handle, &mut size)
            .map_err(|error| format!("No se ha podido leer el tamaño: {error}"))?;
    }
    if size < 0 {
        return Err("El tamaño del archivo no es válido.".into());
    }
    let mut written = FILETIME::default();
    unsafe {
        GetFileTime(handle, None, None, Some(&mut written as *mut FILETIME))
            .map_err(|error| format!("No se ha podido leer la fecha: {error}"))?;
    }
    let mut identity = FILE_ID_INFO::default();
    let identity_ok = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            &mut identity as *mut _ as *mut _,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
        .is_ok()
    };
    Ok(FileFacts {
        size: size as u64,
        modified: filetime_iso(written),
        volume_serial: if identity_ok {
            identity.VolumeSerialNumber
        } else {
            0
        },
        file_id: if identity_ok {
            identity.FileId.Identifier
        } else {
            [0; 16]
        },
    })
}

fn ensure_regular_handle(handle: HANDLE) -> Result<(), String> {
    if is_reparse_handle(handle)? {
        return Err("El origen es un enlace y no se abre.".into());
    }
    Ok(())
}

fn is_reparse_handle(handle: HANDLE) -> Result<bool, String> {
    let mut basic = FILE_BASIC_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileBasicInfo,
            &mut basic as *mut _ as *mut _,
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
        .map_err(|error| format!("No se han podido leer los atributos: {error}"))?;
    }
    Ok((basic.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0) != 0)
}

fn final_path(handle: HANDLE) -> Result<PathBuf, String> {
    let mut buffer = vec![0u16; 32 * 1024];
    let length = unsafe { GetFinalPathNameByHandleW(handle, &mut buffer, FILE_NAME_NORMALIZED) };
    if length == 0 || length as usize >= buffer.len() {
        return Err("No se ha podido confirmar la ruta abierta.".into());
    }
    Ok(PathBuf::from(OsString::from_wide(
        &buffer[..length as usize],
    )))
}

fn rewind(handle: HANDLE) -> Result<(), String> {
    unsafe {
        SetFilePointerEx(handle, 0, None, FILE_BEGIN)
            .map_err(|error| format!("No se ha podido volver al inicio del archivo: {error}"))?;
    }
    Ok(())
}

fn full_path(path: &Path) -> Result<Vec<u16>, String> {
    let wide = wide_path(path);
    let mut buffer = vec![0u16; 32 * 1024];
    let length = unsafe { GetFullPathNameW(PCWSTR(wide.as_ptr()), Some(&mut buffer), None) };
    if length == 0 || length as usize >= buffer.len() {
        return Err("No se ha podido normalizar la ruta.".into());
    }
    Ok(buffer[..length as usize].to_vec())
}

fn nfc(chars: &mut Vec<u16>) -> Result<(), String> {
    let needed = unsafe { NormalizeString(NormalizationC, chars, None) };
    if needed <= 0 {
        return Err("No se ha podido normalizar el texto de la ruta.".into());
    }
    let mut output = vec![0u16; needed as usize];
    let written = unsafe { NormalizeString(NormalizationC, chars, Some(&mut output)) };
    if written <= 0 {
        return Err("No se ha podido normalizar el texto de la ruta.".into());
    }
    output.truncate(written as usize);
    *chars = output;
    Ok(())
}

fn wide_path(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let prefix: Vec<u16> = std::ffi::OsStr::new(r"\\?\").encode_wide().collect();
    if path.is_absolute() && !wide.starts_with(&prefix) && wide.len() >= 240 {
        let mut full = prefix;
        full.append(&mut wide);
        wide = full;
    }
    wide.push(0);
    wide
}

fn filetime_iso(time: FILETIME) -> String {
    let ticks = ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64;
    const EPOCH: u64 = 11_644_473_600_000_0000;
    if ticks < EPOCH {
        return "1601-01-01T00:00:00.0000000Z".into();
    }
    let delta = ticks - EPOCH;
    let secs = delta / 10_000_000;
    let frac = delta % 10_000_000;
    let (year, month, day, hour, minute, second) = civil_from_unix(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{frac:07}Z")
}

pub(crate) fn format_unix_millis(millis: u64) -> String {
    let secs = millis / 1000;
    let fraction = millis % 1000;
    let (year, month, day, hour, minute, second) = civil_from_unix(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction:03}Z")
}

fn civil_from_unix(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = (secs % 86_400) as u32;
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }
    (year as i32, month as u32, day as u32, hour, minute, second)
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_formats_as_iso() {
        let ticks = 11_644_473_600_000_0000u64;
        let time = FILETIME {
            dwLowDateTime: ticks as u32,
            dwHighDateTime: (ticks >> 32) as u32,
        };
        assert_eq!(filetime_iso(time), "1970-01-01T00:00:00.0000000Z");
    }

    #[test]
    fn empty_sha256_matches_the_known_value() {
        let hash = hash_reader(|_| Ok(0)).unwrap();
        assert_eq!(
            hash,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
