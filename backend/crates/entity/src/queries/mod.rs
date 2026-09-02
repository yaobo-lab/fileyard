use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct PageRequest {
    pub page: u64,
    pub per_page: u64,
}

impl PageRequest {
    pub fn normalized(self) -> Self {
        Self {
            page: self.page.max(1),
            per_page: self.per_page.clamp(1, 200),
        }
    }

    pub fn offset(self) -> u64 {
        let value = self.normalized();
        (value.page - 1) * value.per_page
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
