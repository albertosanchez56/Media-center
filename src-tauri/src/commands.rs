/// Prefijo que la interfaz usa para distinguir el fallo provocado de un fallo real del puente.
pub const INTENTIONAL_FAILURE_CODE: &str = "INTENTIONAL_FAILURE";

#[tauri::command]
pub fn bridge_check(fail: bool) -> Result<String, String> {
    if fail {
        Err(format!(
            "{INTENTIONAL_FAILURE_CODE}: Fallo provocado a propósito en Rust"
        ))
    } else {
        Ok("Respuesta de Rust: puente operativo".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_response_when_not_failing() {
        assert_eq!(
            bridge_check(false),
            Ok("Respuesta de Rust: puente operativo".to_string())
        );
    }

    #[test]
    fn returns_intentional_error_when_failing() {
        let error = bridge_check(true).unwrap_err();
        assert!(error.starts_with(&format!("{INTENTIONAL_FAILURE_CODE}: ")));
    }
}
