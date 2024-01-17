fn main() {
    pyo3_build_config::add_extension_module_link_args();

    // will use python from CONDA_PREFIX if it is set
    match get_rpath() {
        Ok(rpath) => {
            println!("cargo:warning=build.rs: setting adding linker -rpath to libpython3.x.[so|dylib|dll] as '{}'", rpath);
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", rpath);
        }
        Err(_) => {
            // println!("cargo:warning=build.rs: {}", err);
        }
    }
}

fn get_rpath() -> Result<String, String> {
    match std::env::var_os("CONDA_PREFIX") {
        Some(path) => {
            println!("cargo:warning=build.rs: using CONDA_PREFIX={:?}", path);
            let rpath = std::path::PathBuf::from(path.clone()).join("lib");
            let rpath = rpath.canonicalize().unwrap_or_else(|_| panic!("Expected $CONDA_PREFIX/lib to be valid path. CONDA_PREFIX={:?}", path));
            Ok(rpath.into_os_string().into_string().unwrap())
        }
        None => Err("build.rs: Failed. CONDA_PREFIX, it is necessary to find correct libpython3.x.[so|dylib|dll] for py03 module".to_owned()),
    }
}
