use super::*;
use pyo3::{
    Bound, Python,
    types::{PyAnyMethods, PyDict, PyModule},
};

fn register_module(py: Python<'_>) -> Bound<'_, PyModule> {
    super::ensure_msgspec_installed(py);
    let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
    tei_rapporteur(py, &module).expect("module registration");
    module
}

#[test]
fn structs_submodule_is_registered() {
    Python::with_gil(|py| {
        let module = register_module(py);
        assert!(
            module
                .hasattr("structs")
                .expect("attribute check should succeed"),
            "structs submodule must be exported"
        );

        let structs = module
            .getattr("structs")
            .expect("structs module should exist");
        assert!(
            structs
                .hasattr("Episode")
                .expect("Episode attribute lookup should succeed"),
            "Episode class must be available for msgspec decoding"
        );
    });
}

#[test]
fn episode_struct_round_trips_messagepack() {
    Python::with_gil(|py| {
        let module = register_module(py);
        let document = Document::try_from_title("Bridgewater")
            .expect("valid title should construct a document");
        let payload: Vec<u8> = module
            .getattr("to_msgpack")
            .expect("to_msgpack export")
            .call1((document.clone(),))
            .expect("to_msgpack call")
            .extract()
            .expect("payload extraction");

        let structs = module.getattr("structs").expect("structs module");
        let episode_type = structs.getattr("Episode").expect("Episode class");
        let msgpack = py
            .import("msgspec.msgpack")
            .expect("msgspec.msgpack import should succeed");
        let decode_kwargs = PyDict::new(py);
        decode_kwargs
            .set_item("type", episode_type)
            .expect("kwargs population");

        let episode = msgpack
            .getattr("decode")
            .expect("decode function")
            .call((payload.clone(),), Some(&decode_kwargs))
            .expect("msgspec decoding should succeed");

        let header = episode
            .getattr("header")
            .expect("Episode should expose header");
        let file_desc = header
            .getattr("file_desc")
            .expect("Header should expose file_desc");
        file_desc
            .setattr("title", "Bridgewater Remix")
            .expect("titles should be mutable");

        let updated_payload: Vec<u8> = msgpack
            .getattr("encode")
            .expect("encode function")
            .call1((episode,))
            .expect("encoding Episode should succeed")
            .extract()
            .expect("payload extraction");
        let round_tripped =
            document_from_msgpack(updated_payload.as_slice()).expect("payload should decode");

        assert_eq!(round_tripped.title().as_str(), "Bridgewater Remix");
    });
}
