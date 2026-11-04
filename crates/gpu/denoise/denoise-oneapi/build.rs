use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=bridge.c");
    
    // On Fedora, check for OIDN in standard locations or oneAPI
    let include_path = PathBuf::from("/usr/include");
    let lib_path = PathBuf::from("/usr/lib64");
    
    // Compile the C++ bridge (OIDN uses C++ headers)
    cc::Build::new()
        .cpp(true)
        .file("bridge.cpp")
        .include(&include_path)
        .flag("-DOIDN_API_EXPORTS")
        .compile("shrimply_oidn_bridge");
    
    // Link against OIDN
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=dylib=OpenImageDenoise");
}
