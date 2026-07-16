//! 自签根 CA 服务(API local-ca)—— list / detail 为真实读取。
//! create / import / export 依赖 rcgen + 密钥落地,在 api 层打桩 501。

use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{certificates, root_cas};
use crate::services::context::CoreContext;
use crate::services::pagination::{Paged, PageParams};
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
    Ok(RootCaRow { root_ca, issued_certificate_count })
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
    Ok(Paged { items, page: page.page, page_size: page.page_size, total })
}

pub async fn get(ctx: &CoreContext, id: &str) -> CoreResult<RootCaRow> {
    let db = &ctx.db;
    let root_ca = root_cas::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::RootCaNotFound, "根 CA 不存在"))?;
    build_row(db, root_ca).await
}
