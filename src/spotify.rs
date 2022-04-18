use std::fmt::Debug;
use reqwest;
use reqwest::{Error, StatusCode};
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::serde_json;
use serde_json::Value;
use crate::spotify::SpotifyError::ReqwestError;
use crate::types::playlist_response;

#[derive(Deserialize, Debug, Clone)]
pub struct SpotifyAuthData {
  // An Access Token that can be provided in subsequent calls, for example to Spotify Web API services.
  access_token: String,
  // How the Access Token may be used: always “Bearer”.
  token_type: String,
  // A space-separated list of scopes which have been granted for this access_token
  scope: String,
  // The time period (in seconds) for which the Access Token is valid.
  expires_in: u32,
  // A token that can be sent to the Spotify Accounts service in place of an authorization code.
  refresh_token: String,
}

impl SpotifyAuthData {
  pub fn new() -> SpotifyAuthData {
    SpotifyAuthData {
      access_token: "".to_string(),
      token_type: "".to_string(),
      scope: "".to_string(),
      expires_in: 0,
      refresh_token: "".to_string(),
    }
  }
}

#[derive(Debug)]
pub struct SpotifyUser {
  pub name: String,
  pub id: String
}

#[derive(Debug)]
pub struct SpotifyPlaylist {
  pub name: String,
  pub id: String
}

#[derive(Debug)]
pub struct SpotifyTrack {
  pub name: String,
  pub id: String
}

pub fn get_tracks(access_token: &str, playlist_id: &str) -> Result<Vec<SpotifyTrack>, SpotifyError> {
  let client = reqwest::blocking::Client::new();
  let mut next_url = Some(format!("https://api.spotify.com/v1/playlists/{}/tracks", playlist_id));
  let mut tracks: Vec<SpotifyTrack> = vec![];
  while let Some(next) = next_url {
    let res = client.get(next)
      .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", access_token))
      .send()?;

    if res.status().is_success() {
      // let json: spotify_types::TracksResponse = res.json()?;
      // next_url = json.next;
      next_url = None;

      println!("{}", res.text()?);

      // let new_items: Vec<SpotifyTrack> = json.items
      //   .iter()
      //   .filter(|p| p.name.is_some() && p.id.is_some())
      //   .map(|p| SpotifyPlaylist {name: p.name.clone().unwrap(), id: p.id.clone().unwrap()})
      //   .collect();
      // playlists.extend(new_items);

    } else {
      return Err(SpotifyError::StatusFailed(res.status()));
    }
  }
  Ok(tracks)
}

pub fn get_playlists(access_token: &str) -> Result<Vec<SpotifyPlaylist>, SpotifyError> {
  let client = reqwest::blocking::Client::new();
  let mut next_url = Some("https://api.spotify.com/v1/me/playlists".to_string());
  let mut playlists: Vec<SpotifyPlaylist> = vec![];
  while let Some(next) = next_url {
    let res = client.get(next)
      .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", access_token))
      .send()?;

    if res.status().is_success() {
      let json: playlist_response::PlaylistResponse = res.json()?;
      next_url = json.next;


      let new_items: Vec<SpotifyPlaylist> = json.items
        .iter()
        .filter(|p| p.name.is_some() && p.id.is_some())
        .map(|p| SpotifyPlaylist {name: p.name.clone().unwrap(), id: p.id.clone().unwrap()})
        .collect();
      playlists.extend(new_items);

    } else {
      return Err(SpotifyError::StatusFailed(res.status()));
    }
  }
  Ok(playlists)
}

pub fn get_user(access_token: &str) -> Result<SpotifyUser, SpotifyError> {
  let client = reqwest::blocking::Client::new();
  let res = client.get("https://api.spotify.com/v1/me")
    .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", access_token))
    .send()?;

  if res.status().is_success() {
    let root: Value = serde_json::from_str(res.text()?.as_str())?;

    let username: &str = root.get("display_name")
      .and_then(|value| value.as_str())
      .ok_or(SpotifyError::InvalidJSON("Username not found in JSON"))?;

    let userid: &str = root.get("id")
      .and_then(|value| value.as_str())
      .ok_or(SpotifyError::InvalidJSON("User ID not found in JSON"))?;

    Ok(SpotifyUser { name: username.to_string(), id: userid.to_string() })
  } else {
    Err(SpotifyError::StatusFailed(res.status()))
  }
}

#[derive(Debug, thiserror::Error)]
pub enum SpotifyError {
  #[error("Reqwest error: {0}")]
  ReqwestError(reqwest::Error),

  #[error("JSON parsing error: {0}")]
  JsonError(serde_json::Error),

  #[error("Invalid JSON: {0}")]
  InvalidJSON(&'static str),

  #[error("Request failed with status {0}")]
  StatusFailed(StatusCode),
}

impl From<reqwest::Error> for SpotifyError {
  fn from(e: reqwest::Error) -> Self {
    ReqwestError(e)
  }
}

impl From<serde_json::Error> for SpotifyError {
  fn from(e: serde_json::Error) -> Self {
    SpotifyError::JsonError(e)
  }
}

#[cfg(test)]
mod tests {
  use reqwest;
  use crate::spotify::{get_playlists, get_tracks, get_user, SpotifyAuthData};

  const TEST_ACCESS_TOKEN: &str = "BQBVS5XO8Vg5BSZ8E43ulJl-FUc0301DxB16_5vQ7Brmwo9X4qf5ehXpTqYMDIBmkb3Mo_0jTOKb5t5R3SGSuTnWnt7AHoo7WklnLmz0ws7Yi00-Z3mT3jpCNn8ozixj-k1ERMdlq9bQk2TfWZ-jWox4PIkQZqn-hutCJBtmPpl30rvwTmtMd5qf";

  #[test]
  fn test_get_user() {
    let user = get_user(TEST_ACCESS_TOKEN).unwrap();
    println!("user = {:?}", user);
    assert_eq!(user.name, "Lars Thiemann");
    assert_eq!(user.id, "11129675811");
  }

  #[test]
  fn test_get_playlist() {
    let playlists = get_playlists(TEST_ACCESS_TOKEN).unwrap();
    println!("playlists = {:?}", playlists);
  }

  #[test]
  fn test_get_tracks() {
    let tracks = get_tracks(TEST_ACCESS_TOKEN, "5PQ9QTvNnAn07WZtnGfiue").unwrap();
    println!("tracks = {:?}", tracks);
  }
}
