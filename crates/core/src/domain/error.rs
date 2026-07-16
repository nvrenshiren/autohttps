//! 领域错误码(snake_case)+ `CoreError` —— 驱动统一错误包络 `{error:{code,message,details}}`。
//!
//! 依据 API common §4.3 + 各模块"错误码清单"。`ErrorCode` 在 core 单一定义并投影到 TS,
//! 前端按 `code` 分支(common §4.1);`message` 仅供展示。HTTP 状态由 `http_status()` 决定。
//! core 不依赖 axum —— api 层读取本类型转 HTTP 响应(保持业务层传输无关)。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 稳定领域错误码 —— 全局 snake_case 唯一(common §4.3:同一 code 不承载不同语义)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // --- 全局(common §4.3)---
    ValidationFailed,
    NotFound,
    InternalError,
    /// 里程碑1 打桩:依赖 ACME/CA/加密的动作尚未落地(non-contract,实现期移除)
    NotImplemented,

    // --- certificates ---
    CertNotFound,
    CertInProgressCannotDelete,
    CertNotExportable,
    InvalidCertState,
    NoDomainsSpecified,
    InvalidDomainReference,
    MultipleWildcardsNotAllowed,
    DomainValidationMethodRequired,
    WildcardRequiresDns01, // 共享规则(certificates/domains)
    IssuanceSourceConflict,
    AcmeAccountRequired,
    InvalidAcmeAccountReference,
    AcmeAccountNotRegistered, // 共享规则(certificates/acme)
    RootCaRequired,
    InvalidRootCaReference,
    RootCaExpired, // 共享规则(certificates/local-ca)
    KeyExportNotAcknowledged,

    // --- domains ---
    DomainNotFound,
    DomainAlreadyExists,
    DomainHasCertificates,
    HostnameImmutable,

    // --- acme ---
    AcmeAccountNotFound, // 共享规则(acme/settings)
    ChallengeNotFound,
    Http01ConfigNotFound,
    AccountStateInvalid,
    ChallengeNotAwaitingManual,
    ChallengeNotRetryable,
    TosNotAgreed,
    InvalidDirectoryUrl,
    NotDns01Challenge,

    // --- local-ca ---
    RootCaNotFound,
    InvalidValidityPeriod,
    ImportKeyMismatch,
    ImportInvalidCertificate,
    ImportKeyDecryptionFailed,

    // --- settings ---
    StoragePathReadOnly,
    SettingNotApplicable,

    // --- tasks ---
    TaskNotFound,
    TaskNotRetryable,
    TaskNotCancellable,
    CertificateDeleted,
}

impl ErrorCode {
    /// HTTP 状态映射(common §4.2)。
    pub fn http_status(self) -> u16 {
        use ErrorCode::*;
        match self {
            ValidationFailed => 400,
            NotFound
            | CertNotFound
            | DomainNotFound
            | AcmeAccountNotFound
            | ChallengeNotFound
            | Http01ConfigNotFound
            | RootCaNotFound
            | TaskNotFound => 404,

            // 409 —— 状态冲突 / 引用冲突
            CertInProgressCannotDelete
            | CertNotExportable
            | InvalidCertState
            | DomainAlreadyExists
            | DomainHasCertificates
            | AccountStateInvalid
            | ChallengeNotAwaitingManual
            | ChallengeNotRetryable
            | TaskNotRetryable
            | TaskNotCancellable
            | CertificateDeleted => 409,

            // 422 —— 业务规则拒绝
            NoDomainsSpecified
            | InvalidDomainReference
            | MultipleWildcardsNotAllowed
            | DomainValidationMethodRequired
            | WildcardRequiresDns01
            | IssuanceSourceConflict
            | AcmeAccountRequired
            | InvalidAcmeAccountReference
            | AcmeAccountNotRegistered
            | RootCaRequired
            | InvalidRootCaReference
            | RootCaExpired
            | KeyExportNotAcknowledged
            | HostnameImmutable
            | TosNotAgreed
            | InvalidDirectoryUrl
            | NotDns01Challenge
            | InvalidValidityPeriod
            | ImportKeyMismatch
            | ImportInvalidCertificate
            | ImportKeyDecryptionFailed
            | StoragePathReadOnly
            | SettingNotApplicable => 422,

            NotImplemented => 501,
            InternalError => 500,
        }
    }
}

/// 领域错误 —— 携带 code + 人读 message + 可选定位 details。
#[derive(Debug, Clone)]
pub struct CoreError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl CoreError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), details: None }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationFailed, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotImplemented, message)
    }

    pub fn http_status(&self) -> u16 {
        self.code.http_status()
    }
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for CoreError {}

/// SeaORM DbErr → 内部错误(不外泄 SQL 细节;详见服务端日志,common §4.3)。
impl From<sea_orm::DbErr> for CoreError {
    fn from(e: sea_orm::DbErr) -> Self {
        tracing::error!(error = %e, "database error");
        CoreError::internal("数据库操作失败")
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
