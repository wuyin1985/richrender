use std::env;
use std::path::PathBuf;

fn build_effect_binding() {
    let export_h = "./../../thirdparty/effekseer/include/export.hpp";
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed='{}'",export_h);
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .header(export_h)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file("src/vfx/bindings.rs")
        .expect("Couldn't write bindings!");
}

fn main() {
    println!("cargo:rustc-link-lib=dylib=RichEffekseer");
    // 在lib目录里搜索本地动态库
    println!("cargo:rustc-link-search=native=./lib");
    build_effect_binding();
}