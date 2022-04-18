use reqwest;
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::serde_json;
use serde_json::Value;

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

pub fn get_user(access_token: &str) -> String {
  let client = reqwest::blocking::Client::new();
  let res = client.get("https://api.spotify.com/v1/me")
    .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", access_token))
    .send().unwrap()
    .text().unwrap();

  let root: Value = serde_json::from_str(res.as_str()).unwrap();

  let username: Option<&str> = root.get("display_name")
    .and_then(|value| value.as_str());

  println!("text: {}", res);

  username.expect("Username is None").to_string()
}

#[cfg(test)]
mod tests {
  use reqwest;
  use crate::spotify::{get_user, SpotifyAuthData};

  const TEST_ACCESS_TOKEN: &str = "BQDBw4TTTR0x_0l64obw2tWFE0uT-dS2URjrjWrK-ddMaZDcLb7ZBHP35-0SrsuuQsN9M1jCMyi93rKCYFQvFgkj2lGq-51tAQRsOu3RdKvg_8UGr4_MfR8O-ij1_3RbYZRH0FOA5xEfMCiZFzUHi4lsKUIKU0qcvdq_QEmdtZI2mWdQKgR7-TgC";

  #[test]
  fn test_get_user() {
    let user = get_user(TEST_ACCESS_TOKEN);
    println!("user = {:?}", user);
    assert_eq!(user, "Lars Thiemann");
  }
}
