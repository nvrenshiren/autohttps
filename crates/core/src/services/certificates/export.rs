//! 证书 PEM 与部署目标导出；由 certificates 模块重导出以保持现有服务 API。

use super::{san_domains, DomainRefData};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{certificates, root_cas};
use crate::services::context::CoreContext;
use sea_orm::EntityTrait;

// ============ 导出(E1,§2.8)============

/// 导出内容(§2.8 `parts`)。`private_key` 是唯一读密钥出口(须 acknowledge)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPart {
    Leaf,
    Chain,
    Fullchain,
    PrivateKey,
}

impl ExportPart {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "leaf" => Some(Self::Leaf),
            "chain" => Some(Self::Chain),
            "fullchain" => Some(Self::Fullchain),
            "private_key" => Some(Self::PrivateKey),
            _ => None,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Leaf => "leaf",
            Self::Chain => "chain",
            Self::Fullchain => "fullchain",
            Self::PrivateKey => "private_key",
        }
    }
}

pub struct ExportInput {
    pub parts: Vec<ExportPart>,
    pub acknowledge_key_export: bool,
}

pub struct ExportBundle {
    pub pem: String,
    pub filename: String,
}

fn push_pem(out: &mut String, pem: &str) {
    let trimmed = pem.trim();
    if trimmed.is_empty() {
        return;
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(trimmed);
    out.push('\n');
}

/// 将保存的证书 PEM 拆成叶子证书与其余证书链。
fn split_certificate_material(material: &str) -> CoreResult<(String, String)> {
    let mut certificates =
        pem::parse_many(material).map_err(|_| CoreError::internal("证书材料 PEM 解析失败"))?;
    if certificates.is_empty()
        || certificates
            .iter()
            .any(|block| block.tag() != "CERTIFICATE")
    {
        return Err(CoreError::internal("证书材料不包含有效的 CERTIFICATE 区块"));
    }

    let leaf = certificates.remove(0);
    Ok((pem::encode(&leaf), pem::encode_many(&certificates)))
}

async fn root_ca_chain(ctx: &CoreContext, root_ca_id: Option<&str>) -> CoreResult<String> {
    let Some(root_ca_id) = root_ca_id else {
        return Ok(String::new());
    };
    root_cas::Entity::find_by_id(root_ca_id)
        .one(&ctx.db)
        .await?
        .map(|root_ca| root_ca.cert_pem)
        .ok_or_else(|| CoreError::internal("证书引用的根 CA 不存在"))
}

/// 导出证书材料(E1)。读 `cert_pem_ref`/`private_key_ref` → age 解密 → 拼 PEM(二进制下载)。
/// - 无文件态(未签发/签发中/签发失败,或缺 `cert_pem_ref`)→ `409 cert_not_exportable`;
/// - 含 `private_key` 未确认 → `422 key_export_not_acknowledged`;
/// - 私钥导出是**唯一读密钥出口**(DTO 仍绝不含 ref)。self_signed 的 `chain` 取其根 CA 证书。
pub async fn export(ctx: &CoreContext, id: &str, input: ExportInput) -> CoreResult<ExportBundle> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    // 无文件态不可导出(§2.8:pending_issue/issuing/issue_failed 或缺文件引用)
    let Some(cert_ref) = cert.cert_pem_ref.as_deref() else {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    };
    if !cert.status.is_exportable() {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    }

    let wants_key = input
        .parts
        .iter()
        .any(|p| matches!(p, ExportPart::PrivateKey));
    if wants_key && !input.acknowledge_key_export {
        return Err(CoreError::new(
            ErrorCode::KeyExportNotAcknowledged,
            "导出私钥需确认敏感数据风险(acknowledgeKeyExport=true)",
        ));
    }

    // age 解密后的材料可能是 ACME 完整证书链;按 PEM 区块拆出叶子与中间链。
    let stored_cert_pem = String::from_utf8(ctx.secrets.load(cert_ref)?)
        .map_err(|_| CoreError::internal("证书文件材料损坏"))?;
    let (leaf_pem, mut chain_pem) = split_certificate_material(&stored_cert_pem)?;
    let root_chain = root_ca_chain(ctx, cert.root_ca_id.as_deref()).await?;
    push_pem(&mut chain_pem, &root_chain);

    // 私钥(敏感 AR4,仅此出口;须已确认)
    let key_pem = if wants_key {
        let key_ref = cert.private_key_ref.as_deref().ok_or_else(|| {
            CoreError::new(ErrorCode::CertNotExportable, "证书无私钥文件,不可导出")
        })?;
        String::from_utf8(ctx.secrets.load(key_ref)?)
            .map_err(|_| CoreError::internal("私钥材料损坏"))?
    } else {
        String::new()
    };

    // 按请求顺序拼接(去重),各 part 之间以换行分隔
    let mut out = String::new();
    let mut seen: Vec<ExportPart> = Vec::new();
    let mut labels: Vec<&str> = Vec::new();
    for p in &input.parts {
        if seen.contains(p) {
            continue;
        }
        seen.push(*p);
        labels.push(p.label());
        match p {
            ExportPart::Leaf => push_pem(&mut out, &leaf_pem),
            ExportPart::Chain => push_pem(&mut out, &chain_pem),
            ExportPart::Fullchain => {
                push_pem(&mut out, &leaf_pem);
                push_pem(&mut out, &chain_pem);
            }
            ExportPart::PrivateKey => push_pem(&mut out, &key_pem),
        }
    }

    let filename = format!("certificate-{id}-{}.pem", labels.join("-"));
    Ok(ExportBundle { pem: out, filename })
}

// ============ 部署目标导出(§2.8 扩展;zip 打包在 api 层)============

/// 部署目标。zip 内文件组织按目标服务的部署格式;全部含私钥(须 acknowledge)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTarget {
    /// fullchain.pem + privkey.pem
    Nginx,
    /// cert.pem + chain.pem + privkey.pem
    Apache,
    /// 单文件 .pfx(PKCS#12,口令必填)
    Iis,
    /// 单文件 .pem(叶子 + 私钥 + 链,HAProxy 合并格式)
    Haproxy,
}

impl ExportTarget {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "nginx" => Some(Self::Nginx),
            "apache" => Some(Self::Apache),
            "iis" => Some(Self::Iis),
            "haproxy" => Some(Self::Haproxy),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Apache => "apache",
            Self::Iis => "iis",
            Self::Haproxy => "haproxy",
        }
    }
}

pub struct ExportFile {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct TargetBundle {
    pub files: Vec<ExportFile>,
    pub zip_name: String,
}

/// 供命名用的主域名(净化非法字符),无域名回退证书 id。
fn export_stem(domains: &[DomainRefData], id: &str) -> String {
    domains
        .first()
        .map(|d| {
            d.hostname.replace(
                |c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'),
                "_",
            )
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("certificate-{id}"))
}

/// PEM → DER(单块 / 多块),材料损坏 → internal。
fn pem_to_der_many(pem_str: &str) -> CoreResult<Vec<Vec<u8>>> {
    pem::parse_many(pem_str)
        .map(|ps| ps.into_iter().map(|p| p.into_contents()).collect())
        .map_err(|_| CoreError::internal("证书材料 PEM 解析失败"))
}

/// PFX(PKCS#12)打包:叶子在前、链在后;3DES 加密以兼容 IIS / Windows 导入。
fn build_pfx(
    stem: &str,
    leaf_pem: &str,
    chain_pem: &str,
    key_pem: &str,
    password: &str,
) -> CoreResult<Vec<u8>> {
    use p12_keystore::{
        Certificate, EncryptionAlgorithm, KeyStore, KeyStoreEntry, PrivateKeyChain,
    };

    let key_der = pem::parse(key_pem)
        .map(|p| p.into_contents())
        .map_err(|_| CoreError::internal("私钥材料 PEM 解析失败"))?;
    let mut certs = Vec::new();
    for der in pem_to_der_many(leaf_pem)? {
        certs.push(
            Certificate::from_der(&der)
                .map_err(|_| CoreError::internal("叶子证书 DER 解析失败"))?,
        );
    }
    for der in pem_to_der_many(chain_pem)? {
        certs.push(
            Certificate::from_der(&der).map_err(|_| CoreError::internal("链证书 DER 解析失败"))?,
        );
    }

    let mut ks = KeyStore::new();
    ks.add_entry(
        stem,
        KeyStoreEntry::PrivateKeyChain(PrivateKeyChain::new(key_der, b"1", certs)),
    );
    ks.writer(password)
        .encryption_algorithm(EncryptionAlgorithm::PbeWithShaAnd3KeyTripleDesCbc)
        .write()
        .map_err(|_| CoreError::internal("PFX 打包失败"))
}

/// 按部署目标导出文件组(E1 扩展)。与 `export` 同规则:无文件态 → 409;全部目标含私钥。
/// `iis` 目标 `pfx_password` 必填(前端已校验,服务层兜底)。
pub async fn export_target(
    ctx: &CoreContext,
    id: &str,
    target: ExportTarget,
    pfx_password: Option<&str>,
) -> CoreResult<TargetBundle> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    let Some(cert_ref) = cert.cert_pem_ref.as_deref() else {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    };
    if !cert.status.is_exportable() {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    }

    let stored_cert_pem = String::from_utf8(ctx.secrets.load(cert_ref)?)
        .map_err(|_| CoreError::internal("证书文件材料损坏"))?;
    let (leaf_pem, mut chain_pem) = split_certificate_material(&stored_cert_pem)?;
    let root_chain = root_ca_chain(ctx, cert.root_ca_id.as_deref()).await?;
    push_pem(&mut chain_pem, &root_chain);
    let key_ref = cert
        .private_key_ref
        .as_deref()
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotExportable, "证书无私钥文件,不可导出"))?;
    let key_pem = String::from_utf8(ctx.secrets.load(key_ref)?)
        .map_err(|_| CoreError::internal("私钥材料损坏"))?;

    let stem = export_stem(&san_domains(db, id).await?, id);
    let pem_of = |parts: &[&str]| -> Vec<u8> {
        let mut s = String::new();
        for p in parts {
            push_pem(&mut s, p);
        }
        s.into_bytes()
    };

    let files: Vec<ExportFile> = match target {
        ExportTarget::Nginx => vec![
            ExportFile {
                name: format!("{stem}.fullchain.pem"),
                data: pem_of(&[&leaf_pem, &chain_pem]),
            },
            ExportFile {
                name: format!("{stem}.privkey.pem"),
                data: pem_of(&[&key_pem]),
            },
        ],
        ExportTarget::Apache => vec![
            ExportFile {
                name: format!("{stem}.cert.pem"),
                data: pem_of(&[&leaf_pem]),
            },
            ExportFile {
                name: format!("{stem}.chain.pem"),
                data: pem_of(&[&chain_pem]),
            },
            ExportFile {
                name: format!("{stem}.privkey.pem"),
                data: pem_of(&[&key_pem]),
            },
        ],
        ExportTarget::Haproxy => vec![ExportFile {
            name: format!("{stem}.pem"),
            data: pem_of(&[&leaf_pem, &key_pem, &chain_pem]),
        }],
        ExportTarget::Iis => {
            let password = pfx_password
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| CoreError::new(ErrorCode::ValidationFailed, "PFX 导出口令必填"))?;
            vec![ExportFile {
                name: format!("{stem}.pfx"),
                data: build_pfx(&stem, &leaf_pem, &chain_pem, &key_pem, password)?,
            }]
        }
    };

    Ok(TargetBundle {
        files,
        zip_name: format!("{stem}-{}.zip", target.label()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acme_fullchain_is_split_into_leaf_and_intermediates() {
        let leaf = pem::encode(&pem::Pem::new("CERTIFICATE", vec![1, 2, 3]));
        let intermediate = pem::encode(&pem::Pem::new("CERTIFICATE", vec![4, 5, 6]));
        let fullchain = format!("{leaf}{intermediate}");

        let (actual_leaf, actual_chain) =
            split_certificate_material(&fullchain).expect("valid PEM blocks");

        let leaf_der = pem::parse(&actual_leaf).expect("leaf PEM").into_contents();
        let chain_der = pem::parse_many(&actual_chain)
            .expect("chain PEM")
            .into_iter()
            .map(|block| block.into_contents())
            .collect::<Vec<_>>();
        assert_eq!(leaf_der, vec![1, 2, 3]);
        assert_eq!(chain_der, vec![vec![4, 5, 6]]);
    }

    /// 生成自签叶子材料(leaf/key PEM)供 PFX 打包测试。
    fn self_signed_materials() -> (String, String) {
        let key = rcgen::KeyPair::generate().expect("keypair");
        let params =
            rcgen::CertificateParams::new(vec!["example.com".to_string()]).expect("params");
        let cert = params.self_signed(&key).expect("self-signed");
        (cert.pem(), key.serialize_pem())
    }

    #[test]
    fn export_stem_sanitizes_hostname() {
        let ds = vec![DomainRefData {
            id: "d1".into(),
            hostname: "*.example.com".into(),
            is_wildcard: true,
        }];
        assert_eq!(export_stem(&ds, "c1"), "_.example.com");
        assert_eq!(export_stem(&[], "c1"), "certificate-c1");
    }

    #[test]
    fn build_pfx_roundtrip() {
        let (leaf_pem, key_pem) = self_signed_materials();
        let pfx = build_pfx("example.com", &leaf_pem, "", &key_pem, "s3cret").expect("build pfx");
        // DER SEQUENCE
        assert_eq!(pfx[0], 0x30);
        let ks = p12_keystore::KeyStore::from_pkcs12(&pfx, "s3cret").expect("parse pfx");
        let (alias, chain) = ks.private_key_chain().expect("key chain entry");
        assert_eq!(alias, "example.com");
        assert_eq!(chain.chain().len(), 1);
        // 口令错误应解不开
        assert!(p12_keystore::KeyStore::from_pkcs12(&pfx, "wrong").is_err());
    }

    #[test]
    fn build_pfx_rejects_bad_pem() {
        assert!(build_pfx("x", "not pem", "", "also not pem", "p").is_err());
    }
}
