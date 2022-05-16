

#[derive(serde::Deserialize, Debug)]
pub struct Page {
    pub auto: Auto,
}

#[derive(serde::Deserialize, Debug)]
pub struct Auto {
   pub pagination: Pagination,
   pub content: Content,
}

#[derive(serde::Deserialize, Debug)]
pub struct Content {
   pub url: String,
}


#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub total_available_items: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_wo_auto_w_other() {
        let data = r#"{
            "other": {
                "id": "name"
            }
        }"#;
        let _page: Page = serde_json::from_str(data).unwrap();
        // assert!(page.auto);
    }

    #[test]
    #[should_panic]
    fn test_w_auto_wo_pagination_w_other() {
        let data = r#"{
            "other": {
                "id": "name"
            },
            "auto": {
                "other": {"a":"b"}
            }
        }"#;
        let _page: Page = serde_json::from_str(data).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_w_auto_w_pagination_wo_items() {
        let data = r#"{
            "other": {
                "id": "name"
            },
            "auto": {
                "other": {"a":"b"},
                "pagination": {
                    "other": 12
                }
            }
        }"#;
        let _: Page = serde_json::from_str(data).unwrap();
    }

    #[test]
    fn test_valid_data_succeeds() {
        let data = r#"{
            "other": {
                "id": "name"
            },
            "auto": {
                "other": {"a":"b"},
                "pagination": {
                    "totalAvailableItems": 12
                }
            }
        }"#;
        let page: Page = serde_json::from_str(data).unwrap();
        assert_eq!(page.auto.pagination.total_available_items, 12);
    }
}
