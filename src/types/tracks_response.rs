use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::serde_json;
use rocket::serde::json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackResponse {
  pub href: Option<String>,
  pub items: Vec<Item>,
  pub limit: i64,
  pub next: Value,
  pub offset: i64,
  pub previous: Value,
  pub total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
  pub added_at: Option<String>,
  pub added_by: AddedBy,
  pub is_local: bool,
  pub primary_color: Value,
  pub track: Track,
  pub video_thumbnail: VideoThumbnail,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddedBy {
  pub external_urls: Option<ExternalUrls>,
  pub href: Option<String>,
  pub id: Option<String>,
  #[serde(rename = "type")]
  pub type_field: Option<String>,
  pub uri: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalUrls {
  pub spotify: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
  pub album: Album,
  pub artists: Vec<Artist>,
  pub available_markets: Vec<String>,
  pub disc_number: i64,
  pub duration_ms: i64,
  pub episode: bool,
  pub explicit: bool,
  pub external_ids: ExternalIds,
  pub external_urls: Option<ExternalUrls>,
  pub href: Option<String>,
  pub id: Option<String>,
  pub is_local: bool,
  pub name: Option<String>,
  pub popularity: i64,
  pub preview_url: Option<String>,
  pub track: bool,
  pub track_number: i64,
  #[serde(rename = "type")]
  pub type_field: Option<String>,
  pub uri: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Album {
  pub album_type: Option<String>,
  pub artists: Vec<Artist>,
  pub available_markets: Vec<String>,
  pub external_urls: Option<ExternalUrls>,
  pub href: Option<String>,
  pub id: Option<String>,
  pub images: Vec<Image>,
  pub name: Option<String>,
  pub release_date: Option<String>,
  pub release_date_precision: Option<String>,
  pub total_tracks: i64,
  #[serde(rename = "type")]
  pub type_field: Option<String>,
  pub uri: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
  pub external_urls: Option<ExternalUrls>,
  pub href: Option<String>,
  pub id: Option<String>,
  pub name: Option<String>,
  #[serde(rename = "type")]
  pub type_field: Option<String>,
  pub uri: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
  pub height: i64,
  pub url: Option<String>,
  pub width: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoThumbnail {
  pub url: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalIds {
  pub isrc: String,
}