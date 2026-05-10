# VRCText Icon Assets

A · EQ Bars 方向 · 主色 #4F9EFF

## 文件清单

- `vrctext.svg` — 主图标，1024×1024，干净 SVG
- `vrctext-glow.svg` — 带外发光的版本（用于网页/官网展示）
- `vrctext.ico` — Windows 多尺寸 ICO（含 16/24/32/48/64/128/256），可直接作为 Rust egui 应用的窗口图标
- `png/vrctext-{N}.png` — 单独的 PNG，N = 16/24/32/48/64/96/128/256/512/1024

## 在 egui 中使用

```rust
// Cargo.toml: image = "0.25"
let icon = image::load_from_memory(include_bytes!("../assets/png/vrctext-256.png"))
    .unwrap()
    .to_rgba8();
let (w, h) = icon.dimensions();
let viewport = egui::ViewportBuilder::default()
    .with_icon(egui::IconData { rgba: icon.into_raw(), width: w, height: h });
```

## Windows 可执行文件嵌入

`build.rs` + `winres` crate 把 `.ico` 嵌入 .exe：

```rust
// build.rs
fn main() {
    if cfg!(target_os = "windows") {
        winres::WindowsResource::new()
            .set_icon("assets/vrctext.ico")
            .compile().unwrap();
    }
}
```
