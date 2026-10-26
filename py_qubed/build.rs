fn main() {
    // When py_qubed is compiled as a cdylib (either as the standalone `import qubed`
    // extension or as a dependency cdylib of another extension like py_qubed_meteo),
    // the Python symbols must be resolved at load time rather than link time on macOS.
    // pyo3's `extension-module` feature normally handles this for the root artifact, but
    // the flag must also be set here so that py_qubed's own cdylib link succeeds when
    // cargo builds it as an intermediate dependency.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
}
