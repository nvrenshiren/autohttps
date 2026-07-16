//! 编译期兜底:确保 rust-embed 的内嵌目录 `frontend/dist` 存在。
//!
//! `src/embed.rs` 的 `#[derive(RustEmbed)] #[folder = "../../frontend/dist"]` 在**编译期**读取该目录;
//! fresh clone 未跑 `npm run build` 时目录不存在,proc-macro 展开会直接报 "folder does not exist"。
//! build script 先于本 crate 编译执行,这里兜底创建空占位目录(+ `.gitkeep`),让 `cargo build`
//! **不依赖先跑前端构建**。真正的 SPA 产物仍由 `npm run build` 生成覆盖;缺产物时运行期回退
//! "请先构建前端" 提示(见 `embed.rs`)。
//!
//! 不发 `rerun-if-changed`:采用 Cargo 默认(包内文件变动即重跑),从而在 `embed.rs` 变更触发
//! proc-macro 重展开的**同一次**构建里,先重建目录再展开,避免目录被清空后编译失败。

use std::path::Path;

fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/dist");
    if let Err(e) = std::fs::create_dir_all(&dist) {
        println!("cargo:warning=无法创建 frontend/dist 占位目录: {e}");
        return;
    }
    let gitkeep = dist.join(".gitkeep");
    if !gitkeep.exists() {
        let _ = std::fs::write(&gitkeep, b"");
    }
}
