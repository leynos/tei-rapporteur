//! Unit tests for the Python-facing bindings (title helpers, dictionary and
//! `MessagePack` exchange, and XML helpers).

use super::*;
use pyo3::{
    Python,
    types::{PyAnyMethods, PyModule},
};
use rmp_serde::to_vec_named;
use serde_json::json;

mod bindings_tests;
mod dict;
mod structs_tests;
mod xml;

fn ensure_msgspec_installed(py: Python<'_>) {
    if py.import("msgspec").is_ok() {
        return;
    }

    let Ok(subprocess) = py.import("subprocess") else { return; };
    let Ok(sys) = py.import("sys") else { return; };
    let Ok(executable) = sys.getattr("executable") else { return; };

    // Bootstrap pip if missing; ignore failures.
    let _pip_bootstrap = subprocess
        .getattr("check_call")
        .and_then(|cc| cc.call1(((executable.clone(), "-m", "ensurepip", "--upgrade"),)));

    let _install_msgspec = subprocess.getattr("check_call").and_then(|cc| {
        cc.call1(((
            executable,
            "-m",
            "pip",
            "install",
            "--break-system-packages",
            "msgspec==0.19.0",
        ),))
    });
}

#[test]
fn document_construction_trims_titles() {
    let document =
        Document::try_from_title("  Wolf 359  ").expect("valid document title should succeed");
    assert_eq!(document.title(), "Wolf 359");
}

#[test]
fn document_construction_rejects_blank_titles() {
    let error = Document::try_from_title("   ").expect_err("blank titles should fail");
    assert!(matches!(error, TeiError::DocumentTitle(_)));
}

#[test]
fn module_registers_python_bindings() {
    Python::with_gil(|py| {
        ensure_msgspec_installed(py);
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");

        for name in [
            "Document",
            "emit_title_markup",
            "from_dict",
            "to_dict",
            "from_msgpack",
            "to_msgpack",
            "parse_xml",
            "emit_xml",
        ] {
            let present = module
                .hasattr(name)
                .expect("module.hasattr should not raise during smoke tests");
            assert!(present, "missing {name}");
        }
    });
}

#[test]
fn python_function_emits_markup() {
    Python::with_gil(|py| {
        ensure_msgspec_installed(py);
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");
        let emit = module
            .getattr("emit_title_markup")
            .expect("emit_title_markup attribute");
        let result: String = emit
            .call1(("Archive 81",))
            .expect("Python call")
            .extract()
            .expect("string extraction");
        assert_eq!(result, "<title>Archive 81</title>");
    });
}

#[test]
fn document_method_emits_markup() {
    let document = Document::try_from_title("King Falls AM").expect("valid doc");
    let markup = document
        .emit_title_markup()
        .expect("method should reuse core helper");
    assert_eq!(markup, "<title>King Falls AM</title>");
}

#[test]
fn from_msgpack_decodes_documents() {
    let fixture =
        TeiDocument::from_title_str("Wolf 359").expect("valid title should build a TeiDocument");
    let payload = to_vec_named(&fixture).expect("MessagePack encoding should succeed");

    let document = from_msgpack(&payload).expect("MessagePack payload should decode");
    assert_eq!(document.title(), "Wolf 359");
}

#[test]
fn from_msgpack_rejects_invalid_payloads() {
    let error = from_msgpack(b"this is not msgpack data")
        .expect_err("invalid payload should surface as an error");
    assert!(error.to_string().contains("invalid MessagePack payload"));
}

#[test]
fn from_msgpack_rejects_empty_payloads() {
    let error = from_msgpack(&[]).expect_err("empty payloads should fail");
    assert!(error.to_string().contains("invalid MessagePack payload"));
}

#[test]
fn from_msgpack_rejects_truncated_payloads() {
    let fixture = TeiDocument::from_title_str("The Magnus Archives")
        .expect("valid title should build a TeiDocument");
    let mut payload = to_vec_named(&fixture).expect("MessagePack encoding should succeed");
    payload.pop();
    let error = from_msgpack(&payload).expect_err("truncated payload should fail");
    assert!(error.to_string().contains("invalid MessagePack payload"));
}

#[test]
fn from_msgpack_rejects_structurally_invalid_documents() {
    let payload = to_vec_named(&json!({ "text": {} }))
        .expect("serialising malformed document should succeed");
    let error =
        from_msgpack(&payload).expect_err("missing header should surface as a decode failure");
    assert!(error.to_string().contains("missing field"));
}

#[test]
fn from_msgpack_rejects_unexpected_types() {
    let payload = to_vec_named(&42u32).expect("serialising primitive should succeed");
    let error = from_msgpack(&payload).expect_err("primitive payload should fail");
    assert!(
        error.to_string().contains("invalid type") || error.to_string().contains("expected struct")
    );
}

#[test]
fn to_msgpack_serialises_documents() {
    let document = Document::try_from_title("Bridgewater").expect("valid document should build");
    let payload = to_msgpack(&document).expect("serialising document should succeed");
    let decoded = document_from_msgpack(payload.as_slice())
        .expect("round-tripping MessagePack should succeed");
    assert_eq!(decoded.title().as_str(), "Bridgewater");
}

#[test]
fn to_msgpack_handles_special_characters() {
    let document = Document::try_from_title(r#"Special <Title> & "Quotes""#)
        .expect("special characters should validate");
    let payload = to_msgpack(&document).expect("serialising document should succeed");
    let decoded =
        document_from_msgpack(payload.as_slice()).expect("decoding MessagePack should succeed");
    assert_eq!(decoded.title().as_str(), r#"Special <Title> & "Quotes""#);
}
