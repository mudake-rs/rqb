use rqb::prelude::Page;
use serde::Serialize;

pub const DEFAULT_LIMIT: u32 = 20;
pub const DEFAULT_OFFSET: u64 = 0;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: u32,
    pub offset: u64,
}

impl<T> From<Page<T>> for PaginatedResponse<T> {
    fn from(page: Page<T>) -> Self {
        Self {
            items: page.items,
            total: page.total,
            limit: page.limit,
            offset: page.offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginated_response_preserves_page_metadata() {
        let response = PaginatedResponse::from(Page {
            items: vec![1, 2],
            total: 10,
            limit: 2,
            offset: 4,
        });

        assert_eq!(response.items, vec![1, 2]);
        assert_eq!(response.total, 10);
        assert_eq!(response.limit, 2);
        assert_eq!(response.offset, 4);
    }
}
