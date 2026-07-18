//! 设置服务(API settings)—— 单例;首次读取即 upsert 默认行(database §2.2)。

use crate::domain::enums::RunMode;
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{acme_accounts, settings};
use crate::services::context::CoreContext;
use crate::util::now_rfc3339;
use sea_orm::*;

pub use crate::persistence::entities::settings::SINGLETON_ID;

/// 服务器形态默认监听端口。
pub const DEFAULT_LISTEN_PORT: i32 = 8443;
/// 服务器形态默认监听地址(仅本机)。
pub const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1";

/// PATCH 输入:外层 None=不改;`Option<Option<T>>` 的 Some(None)=清空。
#[derive(Default)]
pub struct UpdateSettingsInput {
    pub renewal_advance_days: Option<i32>,
    pub auto_renew_enabled: Option<bool>,
    pub default_acme_account_id: Option<Option<String>>,
    pub autostart_enabled: Option<bool>,
    pub listen_address: Option<String>,
    pub listen_port: Option<i32>,
    pub data_storage_path_attempted: bool,
}

/// 读取单例;不存在则按当前形态初始化默认行。
pub async fn get_or_init(ctx: &CoreContext) -> CoreResult<settings::Model> {
    let db = &ctx.db;
    if let Some(m) = settings::Entity::find_by_id(SINGLETON_ID).one(db).await? {
        return Ok(m);
    }
    let now = now_rfc3339();
    let is_desktop = matches!(ctx.run_mode, RunMode::Desktop);
    let model = settings::ActiveModel {
        id: Set(SINGLETON_ID.to_string()),
        renewal_advance_days: Set(30),
        auto_renew_enabled: Set(true),
        default_acme_account_id: Set(None),
        autostart_enabled: Set(if is_desktop { Some(false) } else { None }),
        listen_address: Set((!is_desktop).then(|| DEFAULT_LISTEN_ADDRESS.to_string())),
        listen_port: Set((!is_desktop).then_some(DEFAULT_LISTEN_PORT)),
        data_storage_path: Set(ctx.data_dir.display().to_string()),
        updated_at: Set(now),
    };
    Ok(model.insert(db).await?)
}

pub async fn update(ctx: &CoreContext, input: UpdateSettingsInput) -> CoreResult<settings::Model> {
    let db = &ctx.db;

    // 存储路径只读(SF5)
    if input.data_storage_path_attempted {
        return Err(CoreError::new(
            ErrorCode::StoragePathReadOnly,
            "数据存储路径运行期只读、不可修改",
        ));
    }

    let is_desktop = matches!(ctx.run_mode, RunMode::Desktop);

    // 形态适用校验(仅桌面 / 仅服务器项)
    if input.autostart_enabled.is_some() && !is_desktop {
        return Err(
            CoreError::new(ErrorCode::SettingNotApplicable, "开机自启仅桌面形态适用").with_details(
                serde_json::json!({ "field": "autostartEnabled", "runMode": "server" }),
            ),
        );
    }
    if (input.listen_address.is_some() || input.listen_port.is_some()) && is_desktop {
        return Err(CoreError::new(
            ErrorCode::SettingNotApplicable,
            "监听地址/端口仅服务器形态适用",
        )
        .with_details(serde_json::json!({ "field": "listenAddress", "runMode": "desktop" })));
    }

    // 入参校验
    if let Some(days) = input.renewal_advance_days {
        if days < 1 {
            return Err(CoreError::validation("renewalAdvanceDays 须为 ≥1 的正整数"));
        }
    }
    if let Some(port) = input.listen_port {
        if !(1..=65535).contains(&port) {
            return Err(CoreError::validation("listenPort 越界(1–65535)"));
        }
    }

    // 默认账户校验(共享规则 acme_account_not_found)
    if let Some(Some(account_id)) = &input.default_acme_account_id {
        let exists = acme_accounts::Entity::find_by_id(account_id)
            .one(db)
            .await?;
        if exists.is_none() {
            return Err(
                CoreError::new(ErrorCode::AcmeAccountNotFound, "默认 ACME 账户不存在")
                    .with_details(serde_json::json!({ "id": account_id })),
            );
        }
    }

    let current = get_or_init(ctx).await?;
    let mut active: settings::ActiveModel = current.into();
    if let Some(v) = input.renewal_advance_days {
        active.renewal_advance_days = Set(v);
    }
    if let Some(v) = input.auto_renew_enabled {
        active.auto_renew_enabled = Set(v);
    }
    if let Some(v) = input.default_acme_account_id {
        active.default_acme_account_id = Set(v);
    }
    if let Some(v) = input.autostart_enabled {
        active.autostart_enabled = Set(Some(v));
    }
    if let Some(v) = input.listen_address {
        active.listen_address = Set(Some(v));
    }
    if let Some(v) = input.listen_port {
        active.listen_port = Set(Some(v));
    }
    active.updated_at = Set(now_rfc3339());
    let saved = active.update(db).await?;
    // 内部信号:桌面壳据此即时同步开机自启到 OS(不上 SSE wire)。
    ctx.emit(crate::domain::events::DomainEvent::SettingsChanged);
    Ok(saved)
}
