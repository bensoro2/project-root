use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-search=build");
    println!("cargo:rustc-link-lib=spfresh");
    println!("cargo:rerun-if-changed=src/spfresh.cpp");
    println!("cargo:rerun-if-changed=src/lib.rs");
    
    cc::Build::new()
        .cpp(true)
        .file("src/spfresh.cpp")
        .include("include")
        .warnings(false)
        .compile("spfresh");
        
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let build_path = out_path.parent().unwrap().parent().unwrap().join("build");
    std::fs::create_dir_all(&build_path).unwrap();
}