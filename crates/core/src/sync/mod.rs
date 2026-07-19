//! WebDAV 备份同步模块 —— 手动触发的快照备份/恢复(非实时双向同步;范围定论见评估)。
//!
//! - [`backup`]:一致性 DB 快照(VACUUM INTO)+ secrets 打包 + age 口令加密;
//! - [`webdav`]:WebDAV 迷你客户端(MKCOL/PUT/GET/PROPFIND,hyper-rustls 原生根证书)。

pub mod backup;
pub mod webdav;
