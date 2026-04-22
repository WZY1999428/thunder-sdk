use std::env;
use std::path::PathBuf;

fn main() {
    let lib_path = "D:\\project\\thunder\\lib"; // 你的库路径

    // 1. 告诉 Rust 链接搜索路径 (native 表示原生库)
    println!("cargo:rustc-link-search=native={}", lib_path);

    // 2. 告诉 Rust 链接哪个库 (去掉 .lib 后缀)
    // 注意：如果是动态库，通常链接 .lib 导入库即可
    println!("cargo:rustc-link-lib=dk");

    // 3. 如果使用 bindgen 自动生成绑定
    let bindings = bindgen::Builder::default()
        // 关键：告诉 C 编译器去哪里找头文件 (-I 参数)
        .clang_arg(format!("-I{}", lib_path))
        .header(format!("{}/xl_dl_sdk.h", lib_path))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
