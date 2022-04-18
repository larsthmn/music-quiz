use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::serde_json;
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistResponse {
  pub href: Option<String>,
  pub items: Vec<PlaylistItem>,
  pub limit: i64,
  pub next: Option<String>,
  pub offset: i64,
  pub previous: Value,
  pub total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistItem {
  pub collaborative: bool,
  pub description: Option<String>,
  pub external_urls: Option<ExternalUrls>,
  pub href: Option<String>,
  pub id: Option<String>,
  pub images: Vec<Image>,
  pub name: Option<String>,
  pub owner: Owner,
  pub primary_color: Option<Value>,
  pub public: bool,
  pub snapshot_id: Option<String>,
  pub tracks: Tracks,
  #[serde(rename = "type")]
  pub type_field: Option<String>,
  pub uri: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalUrls {
  pub spotify: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
  pub height: Option<i64>,
  pub url: Option<String>,
  pub width: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Owner {
  pub display_name: Option<String>,
  pub external_urls: Option<ExternalUrls>,
  pub href: Option<String>,
  pub id: Option<String>,
  #[serde(rename = "type")]
  pub type_field: Option<String>,
  pub uri: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tracks {
  pub href: Option<String>,
  pub total: i64,
}