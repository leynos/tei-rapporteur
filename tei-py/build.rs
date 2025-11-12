//! Build script wiring `PyO3`'s configuration into the tei-py crate.

fn main() {
    pyo3_build_config::use_pyo3_cfgs();

    let config = pyo3_build_config::get();
    let building_extension = std::env::var_os("CARGO_FEATURE_EXTENSION_MODULE").is_some();

    if building_extension {
        pyo3_build_config::add_extension_module_link_args();
    } else {
        if let Some(dir) = &config.lib_dir {
            println!("cargo:rustc-link-search=native={dir}");
        }
        if let Some(name) = &config.lib_name {
            println!("cargo:rustc-link-lib={name}");
        }
    }
}
