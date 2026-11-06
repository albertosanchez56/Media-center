//! Formato de mensajes del JSON IPC de mpv: una orden u objeto JSON por línea.

use serde_json::{Value, json};

use super::state::PropertyValue;

#[derive(Debug, PartialEq)]
pub enum IpcMessage {
    Reply {
        request_id: u64,
        error: String,
        data: Value,
    },
    PropertyChange {
        name: String,
        value: PropertyValue,
    },
    StartFile,
    FileLoaded,
    EndFile {
        reason: String,
        error: Option<String>,
    },
    Shutdown,
    Other,
}

pub fn command_line(args: &[&str], request_id: u64) -> String {
    let mut line = json!({ "command": args, "request_id": request_id }).to_string();
    line.push('\n');
    line
}

pub fn observe_line(observer_id: u64, name: &str) -> String {
    let mut line = json!({ "command": ["observe_property", observer_id, name] }).to_string();
    line.push('\n');
    line
}

pub fn parse_line(line: &str) -> IpcMessage {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return IpcMessage::Other;
    };
    if let Some(request_id) = value.get("request_id").and_then(Value::as_u64) {
        return IpcMessage::Reply {
            request_id,
            error: value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("success")
                .to_string(),
            data: value.get("data").cloned().unwrap_or(Value::Null),
        };
    }
    match value.get("event").and_then(Value::as_str) {
        Some("property-change") => IpcMessage::PropertyChange {
            name: value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            value: property_value(value.get("data")),
        },
        Some("start-file") => IpcMessage::StartFile,
        Some("file-loaded") => IpcMessage::FileLoaded,
        Some("end-file") => IpcMessage::EndFile {
            reason: value
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            error: value
                .get("file_error")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        Some("shutdown") => IpcMessage::Shutdown,
        _ => IpcMessage::Other,
    }
}

fn property_value(data: Option<&Value>) -> PropertyValue {
    match data {
        Some(Value::Bool(flag)) => PropertyValue::Flag(*flag),
        Some(Value::Number(number)) => number
            .as_f64()
            .map_or(PropertyValue::Unavailable, PropertyValue::Double),
        _ => PropertyValue::Unavailable,
    }
}

/// Convierte un valor JSON al texto que devolvería `mpv_get_property_string`.
pub fn value_to_property_string(data: &Value) -> Option<String> {
    match data {
        Value::Null => None,
        Value::Bool(true) => Some("yes".to_string()),
        Value::Bool(false) => Some("no".to_string()),
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_command_with_request_id() {
        let line = command_line(&["loadfile", "C:\\a b.mp4"], 7);
        assert!(line.ends_with('\n'));
        let value: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(value["command"], json!(["loadfile", "C:\\a b.mp4"]));
        assert_eq!(value["request_id"], 7);
    }

    #[test]
    fn parses_replies() {
        let message = parse_line(r#"{"data":"0.42.0","error":"success","request_id":3}"#);
        assert_eq!(
            message,
            IpcMessage::Reply {
                request_id: 3,
                error: "success".to_string(),
                data: json!("0.42.0"),
            }
        );
    }

    #[test]
    fn parses_property_changes() {
        assert_eq!(
            parse_line(r#"{"event":"property-change","id":1,"name":"time-pos","data":4.5}"#),
            IpcMessage::PropertyChange {
                name: "time-pos".to_string(),
                value: PropertyValue::Double(4.5),
            }
        );
        assert_eq!(
            parse_line(r#"{"event":"property-change","id":3,"name":"pause","data":true}"#),
            IpcMessage::PropertyChange {
                name: "pause".to_string(),
                value: PropertyValue::Flag(true),
            }
        );
    }

    #[test]
    fn parses_file_events() {
        assert_eq!(
            parse_line(r#"{"event":"file-loaded"}"#),
            IpcMessage::FileLoaded
        );
        assert_eq!(
            parse_line(
                r#"{"event":"end-file","reason":"error","file_error":"unrecognized file format"}"#
            ),
            IpcMessage::EndFile {
                reason: "error".to_string(),
                error: Some("unrecognized file format".to_string()),
            }
        );
    }

    #[test]
    fn converts_values_like_the_string_api() {
        assert_eq!(value_to_property_string(&json!(true)), Some("yes".into()));
        assert_eq!(value_to_property_string(&json!(2)), Some("2".into()));
        assert_eq!(value_to_property_string(&Value::Null), None);
    }
}
