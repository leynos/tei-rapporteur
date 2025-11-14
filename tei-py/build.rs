//! Configures the `PyO3` build based on whether we are compiling the Python
//! extension or running Rust-only targets.
//!
//! When `TEI_PY_BUILD_EXTENSION` (or maturin's `MATURIN_BUILDING`) is set the
//! script emits the dynamic lookup flags required for manylinux wheels so the
//! produced `cdylib` does not link `libpython`. During `cargo test` the flag is
//! absent, so the script instead emits `cargo:rustc-link-*` directives pointing
//! at the host interpreter, allowing `PyO3` to link successfully.

fn main() {
    pyo3_build_config::use_pyo3_cfgs();

    let building_extension = std::env::var_os("TEI_PY_BUILD_EXTENSION")
        .or_else(|| std::env::var_os("MATURIN_BUILDING"))
        .is_some();

    if building_extension {
        pyo3_build_config::add_extension_module_link_args();
    } else {
        let config = pyo3_build_config::get();
        if let Some(dir) = &config.lib_dir {
            println!("cargo:rustc-link-search=native={dir}");
        }
        if let Some(name) = &config.lib_name {
            println!("cargo:rustc-link-lib={name}");
        }
    }
}
