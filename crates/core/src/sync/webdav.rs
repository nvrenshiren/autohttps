//! WebDAV 迷你客户端 —— 仅实现同步需要的四个动作:`MKCOL`(确保目录)、`PUT`(上传)、
//! `GET`(下载)、`PROPFIND`(列文件)。基于 hyper + hyper-rustls(rustls platform-verifier,
//! 原生根证书;依赖已随 instant-acme 在树中,不引入 reqwest)。Basic Auth。
//!
//! 超时收紧(默认 30s);返回结构化错误供上层映射(`WebDavError`)。日志脱敏:不记口令/URL 凭据。

use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use base64::Engine;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Method, Request, StatusCode};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::time::Duration;

/// 请求超时(连接/读取共用)。
const TIMEOUT: Duration = Duration::from_secs(30);

type HttpsClient = Client<hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>;

/// WebDAV 连接配置(口令经调用方从 SecretStore 取,绝不入库/日志)。
#[derive(Clone)]
pub struct WebDavConfig {
    /// 远端目录 URL(如 `https://dav.example.com/dav/autohttps/`;末尾斜杠可选,内部归一)。
    pub base_url: String,
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for WebDavConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 绝不打印口令(AR4/L6 同口径)。
        f.debug_struct("WebDavConfig")
            .field("base_url", &self.base_url)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// 远端文件项(PROPFIND 列出的一个备份文件)。
#[derive(Debug, Clone)]
pub struct RemoteFile {
    /// 文件名(相对 base_url 的最后一段)。
    pub name: String,
    /// 大小(字节;服务端未报则为 None)。
    pub size: Option<u64>,
    /// 服务端报告的修改时间(原样保留,展示用)。
    pub modified: Option<String>,
}

/// 构建带 rustls(平台根证书)+ 超时的 HTTPS client。
fn client() -> CoreResult<HttpsClient> {
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| CoreError::internal(format!("加载系统根证书失败: {e}")))?
        .https_or_http()
        .enable_http1()
        .build();
    Ok(Client::builder(TokioExecutor::new()).build(https))
}

/// 归一 base_url:确保以 `/` 结尾(后续拼文件名)。
fn normalize_base(base: &str) -> String {
    if base.ends_with('/') {
        base.to_string()
    } else {
        format!("{base}/")
    }
}

/// 组装带 Basic Auth 的请求。
fn request(
    cfg: &WebDavConfig,
    method: Method,
    url: &str,
    body: Vec<u8>,
) -> CoreResult<Request<Full<Bytes>>> {
    let auth = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", cfg.username, cfg.password));
    Request::builder()
        .method(method)
        .uri(url)
        .header(hyper::header::AUTHORIZATION, format!("Basic {auth}"))
        .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
        .body(Full::new(Bytes::from(body)))
        .map_err(|e| CoreError::internal(format!("构造 WebDAV 请求失败: {e}")))
}

/// 执行请求并取状态码 + 响应体(超时收口,网络错误映射为 sync_unreachable)。
async fn send(http: &HttpsClient, req: Request<Full<Bytes>>) -> CoreResult<(StatusCode, Vec<u8>)> {
    let fut = http.request(req);
    let resp = tokio::time::timeout(TIMEOUT, fut)
        .await
        .map_err(|_| CoreError::new(ErrorCode::SyncUnreachable, "连接 WebDAV 超时"))?
        .map_err(|e| CoreError::new(ErrorCode::SyncUnreachable, format!("WebDAV 请求失败: {e}")))?;
    let status = resp.status();
    let body = resp
        .into_body()
        .collect()
        .await
        .map_err(|e| {
            CoreError::new(
                ErrorCode::SyncUnreachable,
                format!("读取 WebDAV 响应失败: {e}"),
            )
        })?
        .to_bytes()
        .to_vec();
    Ok((status, body))
}

/// 凭据/权限类错误映射(401/403 → 凭据无效;404 → 路径不存在;其他 → 服务端错误)。
fn map_status(status: StatusCode, what: &str) -> CoreResult<()> {
    match status {
        s if s.is_success() || s == StatusCode::CREATED || s == StatusCode::NO_CONTENT => Ok(()),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(CoreError::new(
            ErrorCode::SyncAuthFailed,
            format!("WebDAV 凭据无效或无权限({what})"),
        )),
        StatusCode::NOT_FOUND => Err(CoreError::new(
            ErrorCode::SyncRemoteNotFound,
            format!("WebDAV 远端路径不存在({what})"),
        )),
        s => Err(CoreError::new(
            ErrorCode::SyncRemoteError,
            format!("WebDAV {what}失败(HTTP {s})"),
        )),
    }
}

/// 测试连接:确保远端目录存在(MKCOL 幂等,405/409 视为已存在)。
pub async fn test_connection(cfg: &WebDavConfig) -> CoreResult<()> {
    ensure_collection(cfg).await
}

/// 确保远端目录存在(MKCOL;405 Method Not Allowed / 409 部分服务端视为"已存在")。
async fn ensure_collection(cfg: &WebDavConfig) -> CoreResult<()> {
    let http = client()?;
    let url = normalize_base(&cfg.base_url);
    let req = request(cfg, Method::from_bytes(b"MKCOL").unwrap(), &url, Vec::new())?;
    let (status, _) = send(&http, req).await?;
    if status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::CONFLICT {
        return Ok(()); // 已存在
    }
    map_status(status, "创建/确认远端目录")
}

/// 上传一个文件(覆盖同名)。返回最终 URL。
pub async fn upload(cfg: &WebDavConfig, remote_name: &str, bytes: Vec<u8>) -> CoreResult<String> {
    ensure_collection(cfg).await?;
    let http = client()?;
    let url = format!("{}{}", normalize_base(&cfg.base_url), remote_name);
    let req = request(cfg, Method::PUT, &url, bytes)?;
    let (status, _) = send(&http, req).await?;
    map_status(status, "上传备份")?;
    Ok(url)
}

/// 下载一个文件。
pub async fn download(cfg: &WebDavConfig, remote_name: &str) -> CoreResult<Vec<u8>> {
    let http = client()?;
    let url = format!("{}{}", normalize_base(&cfg.base_url), remote_name);
    let req = request(cfg, Method::GET, &url, Vec::new())?;
    let (status, body) = send(&http, req).await?;
    map_status(status, "下载备份")?;
    Ok(body)
}

/// 列出远端备份文件(PROPFIND Depth:1;只取非目录的资源项)。
pub async fn list(cfg: &WebDavConfig) -> CoreResult<Vec<RemoteFile>> {
    let http = client()?;
    let url = normalize_base(&cfg.base_url);
    let body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:prop><D:getcontentlength/><D:getlastmodified/><D:resourcetype/></D:prop></D:propfind>"#.to_vec();
    let mut req = request(cfg, Method::from_bytes(b"PROPFIND").unwrap(), &url, body)?;
    req.headers_mut().insert("Depth", "1".parse().unwrap());
    req.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        "application/xml".parse().unwrap(),
    );
    let (status, resp) = send(&http, req).await?;
    map_status(status, "列出远端备份")?;
    Ok(parse_propfind(&resp, &url))
}

/// 解析 PROPFIND 多状态响应(宽松字符串解析:取每个 `<D:response>` 的 href/大小/时间,
/// 跳过目录自身与集合项)。不引 XML 库,WebDAV 服务端输出格式足够规整。
fn parse_propfind(xml: &[u8], base_url: &str) -> Vec<RemoteFile> {
    let text = String::from_utf8_lossy(xml);
    let mut out = Vec::new();
    // 容忍命名空间写法差异(有的服务端用 <d:>、<D:> 或默认命名空间):按标签尾名切分。
    for chunk in text.split(":response>").skip(1) {
        let href = extract_tag(chunk, "href");
        let is_collection = chunk.contains(":collection");
        let size = extract_tag(chunk, "getcontentlength").and_then(|s| s.parse::<u64>().ok());
        let modified = extract_tag(chunk, "getlastmodified");
        let Some(href) = href else { continue };
        if is_collection {
            continue; // 目录项跳过
        }
        // 取最后一段为文件名;剥掉 URL 编码的常用形式(空格等)
        let name = href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let name = name.replace("%20", " ");
        out.push(RemoteFile {
            name,
            size,
            modified,
        });
    }
    out.sort_by(|a, b| b.name.cmp(&a.name)); // 时间戳命名 → 新在前
    let _ = base_url;
    out
}

/// 从 PROPFIND 片段里取某标签尾名的文本(宽松匹配 `<d:href>`/`<D:href>`/`<href>`)。
fn extract_tag(chunk: &str, tag: &str) -> Option<String> {
    // 找 `<x:tag>` 或 `<tag>` 起始
    for pat in [format!(":{tag}>"), format!("<{tag}>")] {
        if let Some(start) = chunk.find(&pat) {
            let content_start = start + pat.len();
            let rest = &chunk[content_start..];
            // 结束标签:`</x:tag>`(`</:` 前缀式匹配,覆盖任意命名空间)或 `</tag>`;
            // 必须以 `</` 开头,否则裸 `:tag>` 会把下一个起始标签的 `<` 吃进来。
            let end = rest
                .find(&format!("</:{tag}>"))
                .or_else(|| rest.find(&format!("</{tag}>")))
                .or_else(|| rest.find("</"))?;
            let value = rest[..end].trim();
            return Some(value.trim_start_matches('/').to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_propfind_collects_files() {
        let xml = br#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/dav/autohttps/</d:href>
    <d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat>
  </d:response>
  <d:response>
    <d:href>/dav/autohttps/backup-20260719.age</d:href>
    <d:propstat><d:prop><d:getcontentlength>12345</d:getcontentlength>
    <d:getlastmodified>Sun, 19 Jul 2026 08:00:00 GMT</d:getlastmodified>
    <d:resourcetype/></d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;
        let files = parse_propfind(xml, "https://x/dav/autohttps/");
        assert_eq!(files.len(), 1, "目录项应被跳过");
        assert_eq!(files[0].name, "backup-20260719.age");
        assert_eq!(files[0].size, Some(12345));
    }
}
