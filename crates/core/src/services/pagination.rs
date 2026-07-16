//! 分页协议(TECH §3.3 / common §3)—— `page`/`pageSize`(默认 20,上限 100)。

pub const DEFAULT_PAGE_SIZE: u64 = 20;
pub const MAX_PAGE_SIZE: u64 = 100;

/// 归一化后的分页参数。
#[derive(Debug, Clone, Copy)]
pub struct PageParams {
    pub page: u64,
    pub page_size: u64,
}

impl PageParams {
    /// 钳制:`page` 最小 1;`pageSize` 默认 20、上限 100(超限钳制,common §3.1)。
    pub fn normalize(page: Option<u64>, page_size: Option<u64>) -> Self {
        let page = page.unwrap_or(1).max(1);
        let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
        Self { page, page_size }
    }

    /// SeaORM `paginate` 用的 0 基页号。
    pub fn zero_based(&self) -> u64 {
        self.page - 1
    }
}

/// 分页结果(api 映射为 `{items,page,pageSize,total}` 包络)。
pub struct Paged<T> {
    pub items: Vec<T>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}
