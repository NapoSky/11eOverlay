//! Wire types mirrored from the ArtyCon overlay API spec.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Persisted (encrypted) token bundle.
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct TokenBundle {
    pub access_token:  String,
    pub refresh_token: String,
    /// Unix epoch (seconds) at which the access token expires.
    pub access_expires_at:  i64,
    /// Unix epoch (seconds) at which the refresh token expires.
    pub refresh_expires_at: i64,
}

/// Response of `/auth/login` and `/auth/refresh`.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token:  String,
    pub refresh_token: String,
    pub access_token_expires_in:  i64,
    pub refresh_token_expires_in: i64,
}

/// Generic API error envelope (`{ "error": { "code": "...", "message": "..." } }`).
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    pub code:    String,
    pub message: String,
}

/// `GET /api/overlay/me`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayUser {
    pub id:        String,
    pub pseudo:    String,
    pub role:      String,
    #[serde(rename = "discordId")]
    pub discord_id: Option<String>,
}

/// `GET /api/overlay/instances`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id:   String,
    pub name: String,
    #[serde(rename = "ownerAccountId")]
    pub owner_account_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// `GET /api/overlay/instances/:id/layers`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id:   String,
    #[serde(rename = "instanceId")]
    pub instance_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// Snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(rename = "layerId")]
    pub layer_id: String,
    pub groups:   Vec<Group>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id:    String,
    pub name:  String,
    pub color: String,
    #[serde(rename = "focusedTargetId", default)]
    pub focused_target_id: Option<String>,
    #[serde(default)]
    pub batteries: Vec<Battery>,
    #[serde(default)]
    pub targets:   Vec<Target>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Battery {
    pub id:       String,
    pub name:     String,
    #[serde(rename = "type")]
    pub kind:     String,
    pub position: Position,
    #[serde(rename = "angleRad", default)]
    pub angle_rad: f64,
    #[serde(rename = "lengthM", default)]
    pub length_m:  f64,
    #[serde(rename = "groupId")]
    pub group_id:  String,
    #[serde(default)]
    pub solutions: Vec<Solution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub id:       String,
    pub position: Position,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind:     Option<String>,
    #[serde(rename = "windForce", default)]
    pub wind_force:     f64,
    #[serde(rename = "windDirection", default)]
    pub wind_direction: f64,
    #[serde(rename = "groupId")]
    pub group_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    #[serde(rename = "targetId")]
    pub target_id: String,
    pub distance:  f64,
    pub angle:     f64,
    #[serde(rename = "windBias")]
    pub wind_bias: Position,
}

/// SSE event payload re-emitted to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ArtyEvent {
    /// Event name: `open`, `presence.changed`, `instance.changed`.
    #[serde(rename = "type")]
    pub kind: String,
    pub data: serde_json::Value,
}

/// Auth status returned to the frontend.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AuthStatus {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pseudo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role:   Option<String>,
}
