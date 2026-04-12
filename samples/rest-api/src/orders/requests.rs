use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::{
    db::{
        ORDER_STATUS,
        orders::{CreateOrder, CreateOrderItem, OrderListQuery, OrderMetadata, OrderPatch},
        schema::order_search_view as order_search,
    },
    error::AppError,
    pagination::{DEFAULT_LIMIT, DEFAULT_OFFSET},
    sort::parse_sort,
};

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OrderListParams {
    #[validate(length(min = 1, max = 32), custom(function = "validate_order_status"))]
    pub status: Option<String>,
    #[validate(length(min = 1, max = 32))]
    pub channel: Option<String>,
    #[validate(range(min = 0))]
    pub min_total: Option<i64>,
    pub sort: Option<String>,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
    #[validate(range(min = 0))]
    pub offset: Option<u64>,
}

impl OrderListParams {
    pub fn limit(&self) -> u32 {
        self.limit.unwrap_or(DEFAULT_LIMIT)
    }

    pub fn offset(&self) -> u64 {
        self.offset.unwrap_or(DEFAULT_OFFSET)
    }

    pub fn into_query(self) -> Result<OrderListQuery, AppError> {
        let limit = self.limit();
        let offset = self.offset();
        let sort = parse_sort(self.sort.as_deref(), order_search::CREATED_AT.desc())?;
        Ok(OrderListQuery {
            status: self.status,
            channel: self.channel,
            min_total: self.min_total,
            sort,
            limit,
            offset,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderRequest {
    pub user_id: Uuid,
    #[validate(length(min = 1, max = 32))]
    pub channel: String,
    #[validate(length(min = 1, max = 32), custom(function = "validate_order_status"))]
    pub status: Option<String>,
    pub metadata: Option<OrderMetadata>,
    #[validate(length(max = 16))]
    pub tags: Option<Vec<String>>,
    #[validate(length(min = 1, max = 20))]
    #[validate(nested)]
    pub items: Vec<CreateOrderItemRequest>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderItemRequest {
    pub product_id: Uuid,
    #[validate(range(min = 1, max = 1000))]
    pub quantity: i32,
    #[validate(range(min = 0))]
    pub unit_price_cents: i64,
    pub metadata: Option<serde_json::Value>,
}

impl From<CreateOrderRequest> for CreateOrder {
    fn from(request: CreateOrderRequest) -> Self {
        Self {
            user_id: request.user_id,
            status: request.status.unwrap_or_else(|| "draft".to_owned()),
            channel: request.channel,
            metadata: request.metadata.unwrap_or_default(),
            tags: request.tags.unwrap_or_default(),
            items: request
                .items
                .into_iter()
                .map(CreateOrderItem::from)
                .collect(),
        }
    }
}

impl From<CreateOrderItemRequest> for CreateOrderItem {
    fn from(request: CreateOrderItemRequest) -> Self {
        Self {
            product_id: request.product_id,
            quantity: request.quantity,
            unit_price_cents: request.unit_price_cents,
            metadata: request.metadata.unwrap_or_else(|| serde_json::json!({})),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_patch_order"))]
#[serde(rename_all = "camelCase")]
pub struct PatchOrderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 32), custom(function = "validate_order_status"))]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 32))]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<OrderMetadata>,
}

impl From<PatchOrderRequest> for OrderPatch {
    fn from(request: PatchOrderRequest) -> Self {
        Self {
            status: request.status,
            channel: request.channel,
            metadata: request.metadata,
        }
    }
}

fn validate_order_status(status: &str) -> Result<(), ValidationError> {
    // ORDER_STATUS comes from CLI-generated schema metadata, so request validation and query
    // validation share the same enum source of truth.
    if ORDER_STATUS.contains(status) {
        Ok(())
    } else {
        let mut error = ValidationError::new("unknown_order_status");
        error.message = Some(format!("unknown order status `{status}`").into());
        Err(error)
    }
}

fn validate_patch_order(patch: &PatchOrderRequest) -> Result<(), ValidationError> {
    // rqb validates field names and values, while the web DTO owns this application-level rule.
    if patch.status.is_some() || patch.channel.is_some() || patch.metadata.is_some() {
        Ok(())
    } else {
        let mut error = ValidationError::new("empty_patch");
        error.message = Some("patch body has no fields".into());
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> Uuid {
        Uuid::parse_str(value).unwrap()
    }

    fn valid_item() -> CreateOrderItemRequest {
        CreateOrderItemRequest {
            product_id: id("40000000-0000-0000-0000-000000000001"),
            quantity: 2,
            unit_price_cents: 1_500,
            metadata: None,
        }
    }

    #[test]
    fn create_order_request_converts_to_db_model_with_defaults() {
        let request = CreateOrderRequest {
            user_id: id("20000000-0000-0000-0000-000000000001"),
            channel: "web".to_owned(),
            status: None,
            metadata: None,
            tags: None,
            items: vec![valid_item()],
        };
        request.validate().unwrap();

        let order = CreateOrder::from(request);
        assert_eq!(order.status, "draft");
        assert_eq!(order.tags, Vec::<String>::new());
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].metadata, serde_json::json!({}));
    }

    #[test]
    fn order_request_validation_rejects_unknown_status_and_empty_patch() {
        let request = CreateOrderRequest {
            user_id: id("20000000-0000-0000-0000-000000000001"),
            channel: "web".to_owned(),
            status: Some("lost".to_owned()),
            metadata: None,
            tags: None,
            items: vec![valid_item()],
        };
        assert!(request.validate().is_err());

        let patch = PatchOrderRequest {
            status: None,
            channel: None,
            metadata: None,
        };
        assert!(patch.validate().is_err());
    }
}
