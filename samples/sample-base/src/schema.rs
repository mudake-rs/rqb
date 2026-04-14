#![allow(dead_code)]
#![allow(clippy::module_inception)]
macro_rules! __rqb_relation_wrapper {
    () => {
        #[derive(Clone, Debug)]
        pub struct Relation {
            inner: rqb::prelude::Relation,
        }
        impl Relation {
            fn new(dataset: Dataset) -> Self {
                Self {
                    inner: rqb::prelude::Relation::new(dataset),
                }
            }
            pub fn alias(mut self, alias: impl Into<String>) -> Self {
                self.inner = self.inner.alias(alias);
                self
            }
            pub fn dataset(&self) -> Dataset {
                self.inner.dataset().clone()
            }
        }
        impl From<Relation> for Dataset {
            fn from(value: Relation) -> Self {
                value.inner.into()
            }
        }
        impl From<&Relation> for Dataset {
            fn from(value: &Relation) -> Self {
                value.dataset()
            }
        }
    };
}
pub mod enums {
    use rqb::prelude::*;
    pub const ORDER_STATUS: EnumType = EnumType::new(
        Some("public"),
        "order_status",
        &["draft", "paid", "cancelled", "refunded"],
    );
    #[derive(Clone, Copy, Debug, PartialEq, Eq, rqb::serde::Serialize, rqb::serde::Deserialize)]
    #[serde(crate = "rqb::serde")]
    pub enum OrderStatus {
        #[serde(rename = "draft")]
        Draft,
        #[serde(rename = "paid")]
        Paid,
        #[serde(rename = "cancelled")]
        Cancelled,
        #[serde(rename = "refunded")]
        Refunded,
    }
    impl OrderStatus {
        pub const fn as_db_str(self) -> &'static str {
            match self {
                Self::Draft => "draft",
                Self::Paid => "paid",
                Self::Cancelled => "cancelled",
                Self::Refunded => "refunded",
            }
        }
    }
    impl DbEnum for OrderStatus {
        const TYPE: EnumType = ORDER_STATUS;
        fn as_db_str(self) -> &'static str {
            OrderStatus::as_db_str(self)
        }
    }
    impl std::fmt::Display for OrderStatus {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.as_db_str())
        }
    }
    impl std::str::FromStr for OrderStatus {
        type Err = &'static str;
        fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
            match value {
                "draft" => Ok(Self::Draft),
                "paid" => Ok(Self::Paid),
                "cancelled" => Ok(Self::Cancelled),
                "refunded" => Ok(Self::Refunded),
                _ => Err("unknown enum variant"),
            }
        }
    }
    pub const USER_STATUS: EnumType =
        EnumType::new(Some("public"), "user_status", &["active", "disabled"]);
    #[derive(Clone, Copy, Debug, PartialEq, Eq, rqb::serde::Serialize, rqb::serde::Deserialize)]
    #[serde(crate = "rqb::serde")]
    pub enum UserStatus {
        #[serde(rename = "active")]
        Active,
        #[serde(rename = "disabled")]
        Disabled,
    }
    impl UserStatus {
        pub const fn as_db_str(self) -> &'static str {
            match self {
                Self::Active => "active",
                Self::Disabled => "disabled",
            }
        }
    }
    impl DbEnum for UserStatus {
        const TYPE: EnumType = USER_STATUS;
        fn as_db_str(self) -> &'static str {
            UserStatus::as_db_str(self)
        }
    }
    impl std::fmt::Display for UserStatus {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.as_db_str())
        }
    }
    impl std::str::FromStr for UserStatus {
        type Err = &'static str;
        fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
            match value {
                "active" => Ok(Self::Active),
                "disabled" => Ok(Self::Disabled),
                _ => Err("unknown enum variant"),
            }
        }
    }
}
pub mod types {
    use rqb::prelude::*;
    pub const UINT_256: TypeSpec = TypeSpec::domain(Some("public"), "uint_256")
        .base(TypeFamily::Numeric)
        .value_repr(ValueRepr::DecimalString)
        .select_repr(SelectRepr::Text);
}
pub mod app_users {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const ORGANIZATION_ID: Field =
        Field::mapped("organizationId", "organization_id", FieldType::Uuid);
    pub const EMAIL: Field = Field::new("email", FieldType::Text);
    pub const STATUS: Field = Field::new("status", FieldType::Enum(super::enums::USER_STATUS));
    pub const PROFILE: Field = Field::new("profile", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const FIELDS: &[Field] = &[
            ID,
            ORGANIZATION_ID,
            EMAIL,
            STATUS,
            PROFILE,
            TAGS,
            CREATED_AT,
        ];
    pub fn dataset() -> Dataset {
        Dataset::static_table("app_users").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn organization_id(&self) -> FieldRef {
            self.inner.field(ORGANIZATION_ID)
        }
        pub fn email(&self) -> FieldRef {
            self.inner.field(EMAIL)
        }
        pub fn status(&self) -> FieldRef {
            self.inner.field(STATUS)
        }
        pub fn profile(&self) -> FieldRef {
            self.inner.field(PROFILE)
        }
        pub fn tags(&self) -> FieldRef {
            self.inner.field(TAGS)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
    }
}
pub mod events {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const ORDER_ID: Field = Field::mapped("orderId", "order_id", FieldType::Uuid);
    pub const EVENT_TYPE: Field = Field::mapped("eventType", "event_type", FieldType::Text);
    pub const PAYLOAD: Field = Field::new("payload", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const FIELDS: &[Field] = &[ID, ORDER_ID, EVENT_TYPE, PAYLOAD, CREATED_AT];
    pub fn dataset() -> Dataset {
        Dataset::static_table("events").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn order_id(&self) -> FieldRef {
            self.inner.field(ORDER_ID)
        }
        pub fn event_type(&self) -> FieldRef {
            self.inner.field(EVENT_TYPE)
        }
        pub fn payload(&self) -> FieldRef {
            self.inner.field(PAYLOAD)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
    }
}
pub mod order_items {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const ORDER_ID: Field = Field::mapped("orderId", "order_id", FieldType::Uuid);
    pub const PRODUCT_ID: Field = Field::mapped("productId", "product_id", FieldType::Uuid);
    pub const QUANTITY: Field = Field::new("quantity", FieldType::Integer);
    pub const UNIT_PRICE_CENTS: Field =
        Field::mapped("unitPriceCents", "unit_price_cents", FieldType::BigInt);
    pub const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const FIELDS: &[Field] = &[
            ID,
            ORDER_ID,
            PRODUCT_ID,
            QUANTITY,
            UNIT_PRICE_CENTS,
            METADATA,
        ];
    pub fn dataset() -> Dataset {
        Dataset::static_table("order_items").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn order_id(&self) -> FieldRef {
            self.inner.field(ORDER_ID)
        }
        pub fn product_id(&self) -> FieldRef {
            self.inner.field(PRODUCT_ID)
        }
        pub fn quantity(&self) -> FieldRef {
            self.inner.field(QUANTITY)
        }
        pub fn unit_price_cents(&self) -> FieldRef {
            self.inner.field(UNIT_PRICE_CENTS)
        }
        pub fn metadata(&self) -> FieldRef {
            self.inner.field(METADATA)
        }
    }
}
pub mod order_search_view {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const EMAIL: Field = Field::new("email", FieldType::Text);
    pub const ORGANIZATION_ID: Field =
        Field::mapped("organizationId", "organization_id", FieldType::Uuid);
    pub const STATUS: Field = Field::new("status", FieldType::Enum(super::enums::ORDER_STATUS));
    pub const STATUS_HISTORY: Field = Field::mapped(
        "statusHistory",
        "status_history",
        FieldType::Array(ElemType::Enum(super::enums::ORDER_STATUS)),
    )
    .sortable(false);
    pub const CHANNEL: Field = Field::new("channel", FieldType::Text);
    pub const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
    pub const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const ITEMS_COUNT: Field = Field::mapped("itemsCount", "items_count", FieldType::BigInt);
    pub const TOTAL_CENTS: Field = Field::mapped("totalCents", "total_cents", FieldType::BigInt);
    pub const FIELDS: &[Field] = &[
            ID,
            EMAIL,
            ORGANIZATION_ID,
            STATUS,
            STATUS_HISTORY,
            CHANNEL,
            TAGS,
            METADATA,
            CREATED_AT,
            ITEMS_COUNT,
            TOTAL_CENTS,
        ];
    pub fn dataset() -> Dataset {
        Dataset::static_view("order_search_view").static_fields(FIELDS)
    }
    pub fn view() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn email(&self) -> FieldRef {
            self.inner.field(EMAIL)
        }
        pub fn organization_id(&self) -> FieldRef {
            self.inner.field(ORGANIZATION_ID)
        }
        pub fn status(&self) -> FieldRef {
            self.inner.field(STATUS)
        }
        pub fn status_history(&self) -> FieldRef {
            self.inner.field(STATUS_HISTORY)
        }
        pub fn channel(&self) -> FieldRef {
            self.inner.field(CHANNEL)
        }
        pub fn tags(&self) -> FieldRef {
            self.inner.field(TAGS)
        }
        pub fn metadata(&self) -> FieldRef {
            self.inner.field(METADATA)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
        pub fn items_count(&self) -> FieldRef {
            self.inner.field(ITEMS_COUNT)
        }
        pub fn total_cents(&self) -> FieldRef {
            self.inner.field(TOTAL_CENTS)
        }
    }
}
pub mod orders {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
    pub const STATUS: Field = Field::new("status", FieldType::Enum(super::enums::ORDER_STATUS));
    pub const STATUS_HISTORY: Field = Field::mapped(
        "statusHistory",
        "status_history",
        FieldType::Array(ElemType::Enum(super::enums::ORDER_STATUS)),
    )
    .sortable(false);
    pub const CHANNEL: Field = Field::new("channel", FieldType::Text);
    pub const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const FIELDS: &[Field] = &[
            ID,
            USER_ID,
            STATUS,
            STATUS_HISTORY,
            CHANNEL,
            METADATA,
            TAGS,
            CREATED_AT,
        ];
    pub fn dataset() -> Dataset {
        Dataset::static_table("orders").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn user_id(&self) -> FieldRef {
            self.inner.field(USER_ID)
        }
        pub fn status(&self) -> FieldRef {
            self.inner.field(STATUS)
        }
        pub fn status_history(&self) -> FieldRef {
            self.inner.field(STATUS_HISTORY)
        }
        pub fn channel(&self) -> FieldRef {
            self.inner.field(CHANNEL)
        }
        pub fn metadata(&self) -> FieldRef {
            self.inner.field(METADATA)
        }
        pub fn tags(&self) -> FieldRef {
            self.inner.field(TAGS)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
    }
}
pub mod organizations {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const SLUG: Field = Field::new("slug", FieldType::Text);
    pub const NAME: Field = Field::new("name", FieldType::Text);
    pub const SETTINGS: Field = Field::new("settings", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const FIELDS: &[Field] = &[ID, SLUG, NAME, SETTINGS, CREATED_AT];
    pub fn dataset() -> Dataset {
        Dataset::static_table("organizations").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn slug(&self) -> FieldRef {
            self.inner.field(SLUG)
        }
        pub fn name(&self) -> FieldRef {
            self.inner.field(NAME)
        }
        pub fn settings(&self) -> FieldRef {
            self.inner.field(SETTINGS)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
    }
}
pub mod pg_type_examples {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const DISPLAY_NAME: Field = Field::mapped("displayName", "display_name", FieldType::Citext);
    pub const PAYLOAD: Field = Field::new("payload", FieldType::Bytea);
    pub const IP_ADDR: Field = Field::mapped("ipAddr", "ip_addr", FieldType::Inet);
    pub const NETWORK: Field = Field::new("network", FieldType::Cidr);
    pub const ACTIVE_WINDOW: Field = Field::mapped(
        "activeWindow",
        "active_window",
        FieldType::Range(ElemType::Timestamptz),
    );
    pub const LOCAL_WINDOW: Field = Field::mapped(
        "localWindow",
        "local_window",
        FieldType::Range(ElemType::Timestamp),
    );
    pub const BILLING_DATES: Field = Field::mapped(
        "billingDates",
        "billing_dates",
        FieldType::Range(ElemType::Date),
    );
    pub const CREATED_LOCAL: Field =
        Field::mapped("createdLocal", "created_local", FieldType::Timestamp);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const FIELDS: &[Field] = &[
            ID,
            DISPLAY_NAME,
            PAYLOAD,
            IP_ADDR,
            NETWORK,
            ACTIVE_WINDOW,
            LOCAL_WINDOW,
            BILLING_DATES,
            CREATED_LOCAL,
            CREATED_AT,
        ];
    pub fn dataset() -> Dataset {
        Dataset::static_table("pg_type_examples").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn display_name(&self) -> FieldRef {
            self.inner.field(DISPLAY_NAME)
        }
        pub fn payload(&self) -> FieldRef {
            self.inner.field(PAYLOAD)
        }
        pub fn ip_addr(&self) -> FieldRef {
            self.inner.field(IP_ADDR)
        }
        pub fn network(&self) -> FieldRef {
            self.inner.field(NETWORK)
        }
        pub fn active_window(&self) -> FieldRef {
            self.inner.field(ACTIVE_WINDOW)
        }
        pub fn local_window(&self) -> FieldRef {
            self.inner.field(LOCAL_WINDOW)
        }
        pub fn billing_dates(&self) -> FieldRef {
            self.inner.field(BILLING_DATES)
        }
        pub fn created_local(&self) -> FieldRef {
            self.inner.field(CREATED_LOCAL)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
    }
}
pub mod products {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const SKU: Field = Field::new("sku", FieldType::Text);
    pub const NAME: Field = Field::new("name", FieldType::Text);
    pub const PRICE_CENTS: Field = Field::mapped("priceCents", "price_cents", FieldType::BigInt);
    pub const ATTRIBUTES: Field = Field::new("attributes", FieldType::Jsonb)
        .sortable(false)
        .json_paths(JsonPathPolicy::Dynamic);
    pub const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const FIELDS: &[Field] = &[
            ID,
            SKU,
            NAME,
            PRICE_CENTS,
            ATTRIBUTES,
            TAGS,
            CREATED_AT,
        ];
    pub fn dataset() -> Dataset {
        Dataset::static_table("products").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn sku(&self) -> FieldRef {
            self.inner.field(SKU)
        }
        pub fn name(&self) -> FieldRef {
            self.inner.field(NAME)
        }
        pub fn price_cents(&self) -> FieldRef {
            self.inner.field(PRICE_CENTS)
        }
        pub fn attributes(&self) -> FieldRef {
            self.inner.field(ATTRIBUTES)
        }
        pub fn tags(&self) -> FieldRef {
            self.inner.field(TAGS)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
    }
}
pub mod withdrawals {
    use rqb::prelude::*;
    pub const ID: Field = Field::new("id", FieldType::Uuid);
    pub const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
    pub const AMOUNT: Field = Field::new("amount", FieldType::Custom(&super::types::UINT_256));
    pub const AMOUNT_HISTORY: Field = Field::mapped(
        "amountHistory",
        "amount_history",
        FieldType::Array(ElemType::Custom(&super::types::UINT_256)),
    )
    .sortable(false);
    pub const WALLET_ADDRESS: Field =
        Field::mapped("walletAddress", "wallet_address", FieldType::Text);
    pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);
    pub const FIELDS: &[Field] = &[
            ID,
            USER_ID,
            AMOUNT,
            AMOUNT_HISTORY,
            WALLET_ADDRESS,
            CREATED_AT,
        ];
    pub fn dataset() -> Dataset {
        Dataset::static_table("withdrawals").static_fields(FIELDS)
    }
    pub fn table() -> Relation {
        Relation::new(dataset())
    }
    __rqb_relation_wrapper!();
    impl Relation {
        pub fn id(&self) -> FieldRef {
            self.inner.field(ID)
        }
        pub fn user_id(&self) -> FieldRef {
            self.inner.field(USER_ID)
        }
        pub fn amount(&self) -> FieldRef {
            self.inner.field(AMOUNT)
        }
        pub fn amount_history(&self) -> FieldRef {
            self.inner.field(AMOUNT_HISTORY)
        }
        pub fn wallet_address(&self) -> FieldRef {
            self.inner.field(WALLET_ADDRESS)
        }
        pub fn created_at(&self) -> FieldRef {
            self.inner.field(CREATED_AT)
        }
    }
}
