use std::env;
use std::fs;
use std::path::PathBuf;

const ICON_PNG: &str = "assets/vrctext/icon-export/png/vrctext-128.png";

fn main() {
    println!("cargo:rerun-if-changed={ICON_PNG}");
    println!("cargo:rerun-if-changed=assets/vrctext/icon-export/vrctext.ico");

    // Decode the icon to raw RGBA at build time so the runtime binary
    // doesn't need to link a PNG decoder.
    let bytes = fs::read(ICON_PNG).expect("read icon png");
    let img = image::load_from_memory(&bytes)
        .expect("decode icon")
        .to_rgba8();
    let (w, h) = img.dimensions();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR set"));
    fs::write(out_dir.join("icon.rgba"), img.into_raw()).expect("write icon.rgba");
    println!("cargo:rustc-env=ICON_W={w}");
    println!("cargo:rustc-env=ICON_H={h}");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/vrctext/icon-export/vrctext.ico");
        res.compile().expect("failed to embed Windows resource");
    }
}
