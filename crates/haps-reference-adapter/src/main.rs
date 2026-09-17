//! Experimental NDJSON adapter for the language-neutral HAPS interoperability suite.
//!
//! This binary exposes the partial canonicalization/hash core and has no formal
//! proof coverage. It is not a HAPS PP or RP implementation or production-ready.
//! Passing its tests does not establish protocol conformance or correct human
//! display/approval. It reads unbounded input lines; deployments would need
//! separate input limits, resource controls and the missing protocol behavior.

#![forbid(unsafe_code)]

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use haps_canon::{canonicalize, parse, ParseError};
use haps_hash::hash_value;
use serde_json::{json, Value};

const PROTOCOL: &str = "haps-conformance-adapter-v1";

fn main() -> ExitCode {
    match serve(io::stdin().lock(), io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("haps-reference-adapter: {error}");
            ExitCode::FAILURE
        }
    }
}

fn serve<R: BufRead, W: Write>(reader: R, mut writer: W) -> io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = process_request(&line);
        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    Ok(())
}

fn process_request(line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(_) => return error_response("", "PROTOCOL_ERROR"),
    };

    let id = match request.get("id").and_then(Value::as_str) {
        Some(id) => id,
        None => return error_response("", "PROTOCOL_ERROR"),
    };
    if request.get("protocol").and_then(Value::as_str) != Some(PROTOCOL) {
        return error_response(id, "PROTOCOL_ERROR");
    }

    match request.get("operation").and_then(Value::as_str) {
        Some("capabilities") => json!({
            "protocol": PROTOCOL,
            "id": id,
            "status": "ok",
            "implementation": {
                "name": "haps-reference-adapter",
                "version": env!("CARGO_PKG_VERSION")
            },
            "haps_versions": ["0.4"],
            "operations": ["canonicalize_and_hash"]
        }),
        Some("canonicalize_and_hash") => {
            let Some(document) = request.get("document").and_then(Value::as_str) else {
                return error_response(id, "PROTOCOL_ERROR");
            };
            match parse(document) {
                Ok(value) => {
                    let bytes = canonicalize(&value);
                    match String::from_utf8(bytes) {
                        Ok(canonical) => json!({
                            "protocol": PROTOCOL,
                            "id": id,
                            "status": "ok",
                            "canonical": canonical,
                            "hash": hash_value(&value)
                        }),
                        Err(_) => error_response(id, "INTERNAL_ERROR"),
                    }
                }
                Err(error) => error_response(id, parse_error_code(error)),
            }
        }
        _ => error_response(id, "UNSUPPORTED_OPERATION"),
    }
}

const fn parse_error_code(error: ParseError) -> &'static str {
    match error {
        ParseError::NonIntegerNumber => "NON_INTEGER_NUMBER",
        ParseError::IntegerOutOfRange => "INTEGER_OUT_OF_RANGE",
        ParseError::DuplicateKey => "DUPLICATE_KEY",
        ParseError::Syntax => "SYNTAX",
        ParseError::TrailingContent => "TRAILING_CONTENT",
        ParseError::DepthExceeded => "DEPTH_EXCEEDED",
    }
}

fn error_response(id: &str, code: &str) -> Value {
    json!({
        "protocol": PROTOCOL,
        "id": id,
        "status": "error",
        "error": { "code": code }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(operation: &str, document: Option<&str>) -> String {
        let mut value = json!({
            "protocol": PROTOCOL,
            "id": "test",
            "operation": operation
        });
        if let Some(document) = document {
            value["document"] = Value::String(document.to_owned());
        }
        value.to_string()
    }

    #[test]
    fn reports_capabilities() {
        let response = process_request(&request("capabilities", None));
        assert_eq!(response["status"], "ok");
        assert_eq!(response["operations"][0], "canonicalize_and_hash");
    }

    #[test]
    fn canonicalizes_and_hashes() {
        let response = process_request(&request(
            "canonicalize_and_hash",
            Some(r#"{ "z": 1, "a": "ok" }"#),
        ));
        assert_eq!(response["status"], "ok");
        assert_eq!(response["canonical"], r#"{"a":"ok","z":1}"#);
        assert!(response["hash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("sha256:")));
    }

    #[test]
    fn exposes_stable_rejection_codes() {
        for (document, code) in [
            (r#"{"amount": 1.5}"#, "NON_INTEGER_NUMBER"),
            (r#"{"amount": 1e2}"#, "NON_INTEGER_NUMBER"),
            (r#"{"a": 1, "a": 2}"#, "DUPLICATE_KEY"),
            (r#"{"n": 9007199254740992}"#, "INTEGER_OUT_OF_RANGE"),
        ] {
            let response = process_request(&request("canonicalize_and_hash", Some(document)));
            assert_eq!(response["status"], "error");
            assert_eq!(response["error"]["code"], code);
        }
    }

    #[test]
    fn rejects_invalid_protocol_requests() {
        let response = process_request("not json");
        assert_eq!(response["error"]["code"], "PROTOCOL_ERROR");
    }
}
