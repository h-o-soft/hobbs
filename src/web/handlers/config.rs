//! Configuration handlers.

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use super::AppState;
use crate::web::dto::ApiResponse;

/// Public site configuration response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SiteConfigResponse {
    /// BBS name.
    pub name: String,
    /// BBS description.
    pub description: String,
    /// SysOp name.
    pub sysop_name: String,
    /// Whether Telnet server is enabled.
    pub telnet_enabled: bool,
}

/// Get public site configuration.
///
/// Returns publicly accessible site configuration such as BBS name.
/// This endpoint does not require authentication.
#[utoipa::path(
    get,
    path = "/api/config/public",
    tag = "Config",
    responses(
        (status = 200, description = "Site configuration", body = ApiResponse<SiteConfigResponse>)
    )
)]
pub async fn get_public_config(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<SiteConfigResponse>> {
    let config = SiteConfigResponse {
        name: state.bbs_name.clone(),
        description: state.bbs_description.clone(),
        sysop_name: state.sysop_name.clone(),
        telnet_enabled: state.telnet_enabled,
    };
    Json(ApiResponse::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_config_response_serialize() {
        let config = SiteConfigResponse {
            name: "Test BBS".to_string(),
            description: "A test BBS".to_string(),
            sysop_name: "Admin".to_string(),
            telnet_enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("Test BBS"));
        assert!(json.contains("telnet_enabled"));
    }
}
