use std::path::PathBuf;

fn main() {
    if let Ok(lib_dir) = std::env::var("SPFRESH_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    } else {
        let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let build_path = out_path.parent().unwrap().parent().unwrap().join("build");
        println!("cargo:rustc-link-search=native={}", build_path.display());
    }
    
    println!("cargo:rustc-link-lib=static=spfresh");
    
    if let Ok(include_dir) = std::env::var("SPFRESH_INCLUDE_DIR") {
        println!("cargo:include={}", include_dir);
    }
    
    println!("cargo:rerun-if-changed=../spfresh/src/spfresh.cpp");
    println!("cargo:rerun-if-changed=../spfresh/include/");
    
    if std::env::var("DOCS_RS").is_err() {
        build_cpp();
    }
}

fn build_cpp() {
    let mut build = cc::Build::new();
    build.cpp(true)
        .include("../spfresh/include")
        .file("../spfresh/src/spfresh.cpp")
        .warnings(false);
        
    build.compile("spfresh");
    
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let build_path = out_path.parent().unwrap().parent().unwrap().join("build");
    std::fs::create_dir_all(&build_path).unwrap();
    
    #[cfg(unix)]
    let lib_name = "libspfresh.a";
    #[cfg(windows)]
    let lib_name = "spfresh.lib";
    
    let src = out_path.join(lib_name);
    let dst = build_path.join(lib_name);
    
    if src.exists() {
        std::fs::copy(&src, &dst).unwrap();
    }
}