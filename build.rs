fn main() {
    println!("cargo:rerun-if-changed=assets/vrctext/icon-export/vrctext.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/vrctext/icon-export/vrctext.ico");
        res.compile().expect("failed to embed Windows resource");
    }
}
