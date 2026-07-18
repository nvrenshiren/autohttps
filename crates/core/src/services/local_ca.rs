//! 自签根 CA 服务(API local-ca)—— list / detail / create / import 为真实实现。
//! 导出(A4)取 `cert_pem`(公开,内联)于 api 层下发;私钥经 age 密文落地、库存 `private_key_ref`。

use crate::domain::enums::RootCaStatus;
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{certificates, root_cas};
use crate::services::context::CoreContext;
use crate::services::pagination::{PageParams, Paged};
use crate::util::{new_id, now_rfc3339};
use sea_orm::*;

pub struct RootCaRow {
    pub root_ca: root_cas::Model,
    pub issued_certificate_count: u64,
}

#[derive(Default)]
pub struct RootCaListFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<crate::domain::enums::RootCaStatus>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

async fn build_row(db: &DatabaseConnection, root_ca: root_cas::Model) -> CoreResult<RootCaRow> {
    let issued_certificate_count = certificates::Entity::find()
        .filter(certificates::Column::RootCaId.eq(&root_ca.id))
        .count(db)
        .await?;
    Ok(RootCaRow {
        root_ca,
        issued_certificate_count,
    })
}

pub async fn list(ctx: &CoreContext, filter: RootCaListFilter) -> CoreResult<Paged<RootCaRow>> {
    let db = &ctx.db;
    let page = PageParams::normalize(filter.page, filter.page_size);
    let mut query = root_cas::Entity::find();
    if let Some(s) = filter.status {
        query = query.filter(root_cas::Column::Status.eq(s));
    }
    let order = matches!(filter.order.as_deref(), Some("desc")).then_some(Order::Desc);
    let (col, default_order) = match filter.sort.as_deref() {
        None | Some("notAfter") => (root_cas::Column::NotAfter, Order::Asc),
        Some("createdAt") => (root_cas::Column::CreatedAt, Order::Desc),
        Some(other) => {
            return Err(CoreError::new(
                ErrorCode::ValidationFailed,
                format!("不支持的排序字段: {other}"),
            ))
        }
    };
    query = query.order_by(col, order.unwrap_or(default_order));

    let paginator = query.paginate(db, page.page_size);
    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page.zero_based()).await?;
    let mut items = Vec::with_capacity(models.len());
    for r in models {
        items.push(build_row(db, r).await?);
    }
    Ok(Paged {
        items,
        page: page.page,
        page_size: page.page_size,
        total,
    })
}

pub async fn get(ctx: &CoreContext, id: &str) -> CoreResult<RootCaRow> {
    let db = &ctx.db;
    let root_ca = root_cas::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::RootCaNotFound, "根 CA 不存在"))?;
    build_row(db, root_ca).await
}

/// 创建入参(A2,L1)。密钥算法等技术参数取 architect 合理默认,不在业务契约暴露。
pub struct CreateRootCaInput {
    pub name: String,
    pub validity_days: i64,
}

/// 导入入参(A3,L2)。`private_key_pem` 敏感:校验配对后 age 密文落地,库存 `private_key_ref`。
pub struct ImportRootCaInput {
    pub name: String,
    pub cert_pem: String,
    pub private_key_pem: String,
    /// 私钥受口令保护时提供;MVP 未支持加密私钥解密(见 ca::parse_and_validate_import)。
    pub key_passphrase: Option<String>,
}

/// A2 / L1:本地生成密钥对并自签(rcgen)→ 私钥 age 密文落地、公开证书内联 → `active`。同步、无过渡态。
pub async fn create(ctx: &CoreContext, input: CreateRootCaInput) -> CoreResult<RootCaRow> {
    let db = &ctx.db;
    let name = input.name.trim();
    if name.is_empty() {
        return Err(CoreError::validation("根 CA 名称不能为空"));
    }
    if input.validity_days <= 0 {
        return Err(CoreError::new(
            ErrorCode::InvalidValidityPeriod,
            "有效期天数须为正整数",
        ));
    }

    let generated = crate::ca::generate_root_ca(name, input.validity_days)?;

    // 私钥 age 密文落数据目录,库内只存引用键(AR4,敏感级最高)
    let key_ref = new_id();
    ctx.secrets.store(&key_ref, generated.key_pem.as_bytes())?;

    let now = now_rfc3339();
    let model = root_cas::ActiveModel {
        id: Set(new_id()),
        name: Set(name.to_string()),
        status: Set(RootCaStatus::Active),
        creation_method: Set("created".to_string()),
        not_before: Set(generated.not_before),
        not_after: Set(generated.not_after),
        serial_number: Set(Some(generated.serial_number)),
        fingerprint: Set(Some(generated.fingerprint)),
        cert_pem: Set(generated.cert_pem),
        private_key_ref: Set(key_ref),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    };
    let root_ca = model.insert(db).await?;
    build_row(db, root_ca).await
}

/// A3 / L2:校验证书↔私钥配对、证书为合法根 CA(x509-parser)→ 落地 → `active`;
/// 导入证书已过有效期则直接判 `expired`(L2)。同步操作。
pub async fn import(ctx: &CoreContext, input: ImportRootCaInput) -> CoreResult<RootCaRow> {
    let db = &ctx.db;
    let name = input.name.trim();
    if name.is_empty() {
        return Err(CoreError::validation("根 CA 名称不能为空"));
    }
    // 校验配对/合法 CA/是否过期;错误码(mismatch/invalid/decryption_failed)由 ca 层给出。
    // 口令保护私钥 MVP 未支持:from_pem 解析失败即 import_key_decryption_failed。
    let _ = &input.key_passphrase;
    let outcome = crate::ca::parse_and_validate_import(&input.cert_pem, &input.private_key_pem)?;

    let key_ref = new_id();
    ctx.secrets
        .store(&key_ref, input.private_key_pem.as_bytes())?;

    let status = if outcome.is_expired {
        RootCaStatus::Expired
    } else {
        RootCaStatus::Active
    };
    let now = now_rfc3339();
    let model = root_cas::ActiveModel {
        id: Set(new_id()),
        name: Set(name.to_string()),
        status: Set(status),
        creation_method: Set("imported".to_string()),
        not_before: Set(outcome.not_before),
        not_after: Set(outcome.not_after),
        serial_number: Set(outcome.serial_number),
        fingerprint: Set(Some(outcome.fingerprint)),
        cert_pem: Set(input.cert_pem),
        private_key_ref: Set(key_ref),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    };
    let root_ca = model.insert(db).await?;
    build_row(db, root_ca).await
}
