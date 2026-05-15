//! Application state shared across all handlers.

use std::sync::Arc;

use rex_domain::BlobStore;
use rex_search::SearchService;

pub struct AppState {
    pub service: Arc<SearchService>,
    pub blobs: Option<Arc<dyn BlobStore>>,
}

impl AppState {
    pub fn builder() -> AppStateBuilder {
        AppStateBuilder::default()
    }
}

#[derive(Default)]
pub struct AppStateBuilder {
    service: Option<Arc<SearchService>>,
    blobs: Option<Arc<dyn BlobStore>>,
}

impl AppStateBuilder {
    pub fn service(mut self, s: Arc<SearchService>) -> Self {
        self.service = Some(s);
        self
    }

    pub fn blobs(mut self, b: Arc<dyn BlobStore>) -> Self {
        self.blobs = Some(b);
        self
    }

    pub fn build(self) -> Result<AppState, &'static str> {
        Ok(AppState {
            service: self.service.ok_or("AppState: service is required")?,
            blobs: self.blobs,
        })
    }
}
