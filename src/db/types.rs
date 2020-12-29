use postgres_types::{FromSql, ToSql};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromSql, ToSql)]
#[postgres(name = "feature")]
pub struct Feature {
    pub(crate) name: String,
    pub(crate) subfeatures: Vec<String>,
    /// [`None`] when this crate was built before optional dependencies were tracked
    pub(crate) optional_dependency: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, FromSql, ToSql)]
#[postgres(name = "build_status")]
pub(crate) enum BuildStatus {
    #[postgres(name = "success")]
    Success,
    #[postgres(name = "failure")]
    Failure,
    #[postgres(name = "in_progress")]
    InProgress,
}

impl Feature {
    pub fn new(name: String, subfeatures: Vec<String>, optional_dependency: bool) -> Self {
        Feature {
            name,
            subfeatures,
            optional_dependency: Some(optional_dependency),
        }
    }

    pub fn is_private(&self) -> bool {
        self.name.starts_with('_')
    }
}
