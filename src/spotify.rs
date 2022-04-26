use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use rocket::http::private::cookie::Expiration::DateTime;
use rspotify::AuthCodeSpotify;
use rspotify::clients::{BaseClient, OAuthClient};
use crate::{GamePreferences, GameReferences, GameState};
use chrono::prelude::*;


pub fn spotify_loop(state: Arc<Mutex<GameState>>, preferences: Arc<Mutex<GamePreferences>>,
                references: Arc<Mutex<GameReferences>>) {
  loop {
    // Always lock references fist to avoid deadlock!
    let r = references.lock().unwrap();
    // Refresh token
    let mut needs_refresh = false;
    if let Some(token) = r.spotify_client.get_token().lock().unwrap().as_ref() {
      if Utc::now() + chrono::Duration::seconds(30) > token.expires_at.expect("Token has no expiration") {
        needs_refresh = true;
      }
      println!("now: {:?}, expires at {:?}, {:?}", Utc::now() + chrono::Duration::seconds(30), token.expires_at.expect("Token has no expiration"), needs_refresh);
    }
    if needs_refresh {
      match r.spotify_client.refresh_token() {
        Ok(_) => println!("Refreshed spotify token!"),
        Err(e) => eprintln!("Error on refreshing token: {:?}", e)
      }
    }

    // Refresh playlists
    if r.spotify_client.is_authenticated() {
      let mut p = preferences.lock().unwrap();
      p.playlists = r.spotify_client.current_user_playlists()
        .filter_map(|playlist| playlist.ok())
        .map(|playlist|playlist.name)
        .collect();
    }

    drop(r);
    std::thread::sleep(Duration::from_secs(20));
  }
}

pub trait CustomSpotifyChecks {
  fn is_authenticated(&self) -> bool;
}

impl CustomSpotifyChecks for AuthCodeSpotify {
  fn is_authenticated(&self) -> bool {
    self.get_token().lock().unwrap().is_some()
  }
}