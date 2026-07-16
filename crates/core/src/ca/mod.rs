//! 自签 CA 原语(TECH 决策5:rcgen 0.14,纯 Rust、避 openssl,ring 后端)。
//!
//! 三件事:① 本地生成自签**根 CA**(创建,L1);② 用根 CA 签发**内网叶子证书**(执行器 self_signed
//! issue/renew,受托 B1);③ 解析并校验**导入**的根 CA(证书↔私钥配对 + 是否合法 CA + 是否已过期,L2)。
//!
//! 私钥材料(PEM)由调用方交 `secrets` age 加密落地,**本模块不落盘、不入库**(AR4)。
//! 作废(B2)不产 CRL/OCSP,走 `internal_cert_revocations` 本地作废记录(决策5 注),不在本模块。

use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use rcgen::{
    CertificateParams, DnType, ExtendedKeyUsagePurpose, Issuer, IsCa, KeyPair, KeyUsagePurpose,
    PublicKeyData, SanType, SerialNumber,
};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

/// 一份签发产物 —— 公开证书 PEM + 私钥 PEM(敏感,交 secrets 加密)+ 标识/有效期。
pub struct GeneratedCert {
    pub cert_pem: String,
    /// 私钥 PEM(PKCS#8)—— 敏感 AR4,交 `secrets` 加密落地。
    pub key_pem: String,
    pub serial_number: String,
    pub fingerprint: String,
    pub not_before: String,
    pub not_after: String,
}

/// 导入根 CA 的校验产物(不含私钥;私钥另交 secrets)。
pub struct ImportOutcome {
    pub serial_number: Option<String>,
    pub fingerprint: String,
    pub not_before: String,
    pub not_after: String,
    /// 导入证书本身已过有效期 → 落 `expired`(L2)。
    pub is_expired: bool,
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn to_rfc3339(dt: OffsetDateTime) -> String {
    dt.format(&Rfc3339).unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

/// 随机 16 字节正序列号(清最高位避免被解析为负数)。
fn random_serial() -> SerialNumber {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes[0] &= 0x7f;
    if bytes[0] == 0 {
        bytes[0] = 0x01;
    }
    SerialNumber::from_slice(&bytes)
}

/// 十六进制大写冒号分隔(证书标识展示惯例)。
fn hex_colon(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

/// SHA-256 指纹(证书 DER),十六进制大写冒号分隔。
fn fingerprint_of(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    hex_colon(&digest)
}

fn internal<E: std::fmt::Display>(what: &str) -> impl Fn(E) -> CoreError + '_ {
    move |e| CoreError::internal(format!("{what}: {e}"))
}

/// ① 本地生成自签根 CA(创建,L1)。CommonName = 名称;有效期自 now 起 `validity_days` 天。
pub fn generate_root_ca(name: &str, validity_days: i64) -> CoreResult<GeneratedCert> {
    let key = KeyPair::generate().map_err(internal("生成根 CA 密钥失败"))?;

    let nb = now();
    let na = nb + Duration::days(validity_days);
    let serial = random_serial();
    let serial_str = serial_string(&serial);

    let mut params = CertificateParams::default();
    params.not_before = nb;
    params.not_after = na;
    params.serial_number = Some(serial);
    params.distinguished_name = {
        let mut dn = rcgen::DistinguishedName::new();
        dn.push(DnType::CommonName, name);
        dn
    };
    params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages =
        vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign, KeyUsagePurpose::DigitalSignature];

    let cert = params.self_signed(&key).map_err(internal("自签根 CA 失败"))?;
    Ok(GeneratedCert {
        cert_pem: cert.pem(),
        key_pem: key.serialize_pem(),
        serial_number: serial_str,
        fingerprint: fingerprint_of(cert.der()),
        not_before: to_rfc3339(nb),
        not_after: to_rfc3339(na),
    })
}

/// ② 用根 CA 签发内网叶子证书(执行器 self_signed issue/renew,受托 B1)。
///
/// `hostnames` 为 SAN(至少一个);CommonName 取首个。有效期自 now 起 `validity_days` 天。
pub fn sign_leaf(
    root_cert_pem: &str,
    root_key_pem: &str,
    hostnames: &[String],
    validity_days: i64,
) -> CoreResult<GeneratedCert> {
    if hostnames.is_empty() {
        return Err(CoreError::internal("签发叶子证书缺少域名"));
    }
    let issuer_key = KeyPair::from_pem(root_key_pem).map_err(internal("加载根 CA 私钥失败"))?;
    let issuer = Issuer::from_ca_cert_pem(root_cert_pem, issuer_key)
        .map_err(internal("加载根 CA 证书失败"))?;

    let leaf_key = KeyPair::generate().map_err(internal("生成叶子密钥失败"))?;

    let nb = now();
    let na = nb + Duration::days(validity_days);
    let serial = random_serial();
    let serial_str = serial_string(&serial);

    let mut params = CertificateParams::default();
    params.not_before = nb;
    params.not_after = na;
    params.serial_number = Some(serial);
    params.distinguished_name = {
        let mut dn = rcgen::DistinguishedName::new();
        dn.push(DnType::CommonName, hostnames[0].clone());
        dn
    };
    params.is_ca = IsCa::NoCa;
    params.key_usages =
        vec![KeyUsagePurpose::DigitalSignature, KeyUsagePurpose::KeyEncipherment];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let mut sans = Vec::with_capacity(hostnames.len());
    for h in hostnames {
        let name = SanType::DnsName(
            h.as_str().try_into().map_err(internal("非法 SAN 域名"))?,
        );
        sans.push(name);
    }
    params.subject_alt_names = sans;

    let cert = params.signed_by(&leaf_key, &issuer).map_err(internal("签发叶子证书失败"))?;
    Ok(GeneratedCert {
        cert_pem: cert.pem(),
        key_pem: leaf_key.serialize_pem(),
        serial_number: serial_str,
        fingerprint: fingerprint_of(cert.der()),
        not_before: to_rfc3339(nb),
        not_after: to_rfc3339(na),
    })
}

/// ③ 解析并校验导入的根 CA(L2):证书↔私钥配对 · 是否合法 CA · 是否已过期。
///
/// - 证书非合法 CA → `import_invalid_certificate`。
/// - 私钥无法解析(含受口令保护的加密私钥,MVP 未支持解密)→ `import_key_decryption_failed`。
/// - 证书与私钥不配对 → `import_key_mismatch`。
pub fn parse_and_validate_import(cert_pem: &str, key_pem: &str) -> CoreResult<ImportOutcome> {
    // 解析证书 PEM → DER
    let pem = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
        .map_err(|_| CoreError::new(ErrorCode::ImportInvalidCertificate, "无法解析根 CA 证书 PEM"))?
        .1;
    let der = pem.contents.as_slice();
    let (_rest, x509) = x509_parser::parse_x509_certificate(der).map_err(|_| {
        CoreError::new(ErrorCode::ImportInvalidCertificate, "无法解析根 CA 证书")
    })?;

    // 必须是 CA 证书
    if !x509.is_ca() {
        return Err(CoreError::new(
            ErrorCode::ImportInvalidCertificate,
            "该证书不是合法的 CA 证书(缺 BasicConstraints CA)",
        ));
    }

    // 加载私钥(受口令保护的加密私钥 MVP 未支持 → 判解密失败)
    let key = KeyPair::from_pem(key_pem).map_err(|_| {
        CoreError::new(
            ErrorCode::ImportKeyDecryptionFailed,
            "无法解析私钥(受口令保护的加密私钥暂不支持,请提供未加密 PEM)",
        )
    })?;

    // 配对校验:私钥公钥 SPKI DER == 证书 SubjectPublicKeyInfo DER
    let key_spki = key.subject_public_key_info();
    let cert_spki = x509.public_key().raw;
    if key_spki.as_slice() != cert_spki {
        return Err(CoreError::new(ErrorCode::ImportKeyMismatch, "证书与私钥不配对"));
    }

    // 确认可用作签发根(rcgen 能据其构造 Issuer)
    Issuer::from_ca_cert_pem(cert_pem, key)
        .map_err(|_| CoreError::new(ErrorCode::ImportInvalidCertificate, "证书不可用作签发根 CA"))?;

    let nb = x509.validity().not_before.to_datetime();
    let na = x509.validity().not_after.to_datetime();
    let is_expired = na < now();

    Ok(ImportOutcome {
        serial_number: Some(hex_colon(x509.raw_serial())),
        fingerprint: fingerprint_of(der),
        not_before: to_rfc3339(nb),
        not_after: to_rfc3339(na),
        is_expired,
    })
}

/// rcgen `SerialNumber` 无公开取字节接口时,用其 DER/ to_string 做展示。
fn serial_string(serial: &SerialNumber) -> String {
    // rcgen SerialNumber 实现 Display(十进制)。这里取其字节做十六进制冒号展示,与导入路径口径一致。
    hex_colon(serial.as_ref())
}
