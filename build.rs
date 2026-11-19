use std::fs;

fn main() {
    let mut build = cc::Build::new();

    for entry in fs::read_dir("doom").unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        if !name.ends_with(".c") {
            continue;
        }

        if name.starts_with("doomgeneric_") || name.contains("sdl") || name.contains("allegro") {
            continue;
        }
        build.file(&path);
    }

    build
        .include("doom")
        .include("libc/include")
        .compiler("clang")
        .target("x86_64-unknown-none")
        .opt_level(2)
        .flag("-ffreestanding")
        .flag("-nostdlibinc")
        .flag("-fno-stack-protector")
        .flag("-mno-red-zone")
        .flag("-mno-sse")
        .flag("-mno-mmx")
        .flag("-fno-pic")
        .flag("-w")
        .define("CMAP256", None)
        .define("DOOMGENERIC_RESX", "320")
        .define("DOOMGENERIC_RESY", "200")
        .compile("doom");

    println!("cargoLrerun-if-changed=doom");
    println!("cargo:rerun-if-changed=libc");
    println!("cargo:rustc-link-arg=--error-limit=0");
}