#![deny(unsafe_code)]
use photocraft_plugin_sdk::api::{
    ApiError, CommandInvocation, ErrorCode, EventRequest, PanelResponse, Result,
};
use photocraft_plugin_sdk::serde_json::{Value, json};
pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

fn handle(request: &EventRequest) -> Result<PanelResponse> {
    let mut response = PanelResponse {
        state: request.state.clone(),
        message: "BRIA RMBG 1.4 · Local background removal. Non-commercial model license; commercial use requires a BRIA agreement.".into(),
        ..PanelResponse::default()
    };
    if request.event == "create" {
        if !request.document.is_object() {
            return Err(ApiError::new(
                ErrorCode::InvalidJson,
                "Open an image before creating a result.",
            ));
        }
        let output = match request.values.get("output") {
            None => "foreground",
            Some(Value::String(value))
                if ["foreground", "background", "mask"].contains(&value.as_str()) =>
            {
                value
            }
            _ => {
                return Err(ApiError::new(
                    ErrorCode::InvalidJson,
                    "Choose foreground, background, or mask.",
                ));
            }
        };
        response.commands.push(CommandInvocation {
            command: "plugin.processImage".into(),
            params: json!({"processor":"native", "options":{"output":output}, "preview":true})
                .as_object()
                .cloned()
                .ok_or_else(|| {
                    ApiError::new(ErrorCode::InvalidJson, "Invalid processing request.")
                })?,
        });
        response.message = "Creating BRIA result…".into();
    }
    Ok(response)
}

photocraft_plugin_sdk::export_plugin!(bindings::MANIFEST_JSON, handle);

#[cfg(test)]
mod tests {
    use super::*;

    fn request(event: &str, values: Value, document: Value) -> EventRequest {
        photocraft_plugin_sdk::serde_json::from_value(
            json!({"apiVersion":2,"host":{"version":1,"capabilities":[]},
            "event":event,"values":values,"document":document,"state":{"retained":true}}),
        )
        .unwrap()
    }

    #[test]
    fn actions_request_only_owned_native_preview_and_preserve_state() {
        for output in ["foreground", "background", "mask"] {
            let request = request(
                "create",
                json!({"output":output}),
                json!({"width":10,"height":10}),
            );
            let reply = handle(&request).unwrap();
            assert_eq!(reply.state, request.state);
            assert_eq!(reply.commands.len(), 1);
            assert_eq!(
                reply.commands[0].params,
                json!({"processor":"native","options":{"output":output},"preview":true})
                    .as_object()
                    .unwrap()
                    .clone()
            );
            assert!(reply.requests.is_empty());
        }
    }

    #[test]
    fn observers_unknown_events_and_invalid_actions_never_process() {
        for event in [
            "init",
            "change",
            "document.changed",
            "host_response",
            "unrecognized",
        ] {
            assert!(
                handle(&request(event, json!({}), Value::Null))
                    .unwrap()
                    .commands
                    .is_empty()
            );
        }
        assert!(handle(&request("create", json!({"output":"other"}), json!({}))).is_err());
        assert!(handle(&request("create", json!({}), Value::Null)).is_err());
    }
}
