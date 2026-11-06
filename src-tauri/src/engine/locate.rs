use std::ffi::OsString;
use std::path::{Path, PathBuf};

const LIBMPV_VARIABLE: &str = "MEDIA_CENTER_LIBMPV";
const MPV_VARIABLE: &str = "MEDIA_CENTER_MPV";

pub fn libmpv() -> Result<PathBuf, String> {
    resolve(
        LIBMPV_VARIABLE,
        std::env::var_os(LIBMPV_VARIABLE),
        vendor_dir().join("libmpv").join("libmpv-2.dll"),
        Path::is_file,
    )
}

pub fn mpv_executable() -> Result<PathBuf, String> {
    resolve(
        MPV_VARIABLE,
        std::env::var_os(MPV_VARIABLE),
        vendor_dir().join("mpv").join("mpv.exe"),
        Path::is_file,
    )
}

/// Los binarios de prueba viven fuera de Git, en `src-tauri/vendor/mpv`.
fn vendor_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("mpv")
}

fn resolve(
    variable: &str,
    override_path: Option<OsString>,
    default_path: PathBuf,
    exists: impl Fn(&Path) -> bool,
) -> Result<PathBuf, String> {
    let candidate = override_path.map_or(default_path, PathBuf::from);
    if exists(&candidate) {
        Ok(candidate)
    } else {
        Err(format!(
            "No se encuentra {}. Colócalo ahí o indica otra ruta en la variable {variable}.",
            candidate.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_default_path_without_override() {
        let found = resolve("VAR", None, PathBuf::from("a/b.dll"), |_| true);
        assert_eq!(found, Ok(PathBuf::from("a/b.dll")));
    }

    #[test]
    fn prefers_the_override_path() {
        let found = resolve(
            "VAR",
            Some(OsString::from("c/d.dll")),
            PathBuf::from("a/b.dll"),
            |_| true,
        );
        assert_eq!(found, Ok(PathBuf::from("c/d.dll")));
    }

    #[test]
    fn explains_how_to_fix_a_missing_binary() {
        let error = resolve("VAR", None, PathBuf::from("a/b.dll"), |_| false).unwrap_err();
        assert!(error.contains("b.dll"));
        assert!(error.contains("VAR"));
    }
}
