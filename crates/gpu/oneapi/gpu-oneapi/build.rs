use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=bridge.c");
    
    // On Fedora, Level Zero is installed in standard system locations
    let include_path = PathBuf::from("/usr/include");
    let lib_path = PathBuf::from("/usr/lib64");
    
    // Compile the C bridge
    cc::Build::new()
        .file("bridge.c")
        .include(&include_path)
        .flag("-DZE_API_EXPORTS")
        .compile("shrimply_oneapi_bridge");
    
    // Link against Level Zero (Fedora package names)
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=dylib=ze_loader");
    println!("cargo:rustc-link-lib=dylib=ze_intel_gpu");
}
