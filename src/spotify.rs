use std::borrow::Borrow;
use std::sync::{Arc, Mutex};
use std::time::{Duration};
use rspotify::AuthCodeSpotify;
use rspotify::clients::{BaseClient, OAuthClient};
use crate::{GamePreferences, GameReferences, GameState};
use chrono::prelude::*;
use rspotify::model::Id;
use crate::game::Playlist;


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
    }
    if needs_refresh {
      match r.spotify_client.refresh_token() {
        Ok(_) => log::info!("Refreshed spotify token!"),
        Err(e) => log::warn!("Error on refreshing token: {:?}", e)
      }
    }

    // Refresh playlists
    if r.spotify_client.has_token() {
      let mut p = preferences.lock().unwrap();
      p.playlists = r.spotify_client.current_user_playlists()
        .filter_map(|playlist| playlist.ok())
        .map(|playlist| Playlist { name: playlist.name, id: playlist.id.uri() })
        .collect();
      // Select a playlist if none is selected or selected one does not exist
      if (p.selected_playlist.is_none()
        || p.playlists.iter().find(|x| x.id == p.selected_playlist.as_ref().unwrap().id).is_none())
        && p.playlists.len() > 0 {
        log::info!("set selected playlist to first one {:?}", p.playlists[0]);
        p.selected_playlist = Some(p.playlists[0].clone());
      }
    }

    drop(r);
    std::thread::sleep(Duration::from_secs(20));
  }
}

pub trait CustomSpotifyChecks {
  fn has_token(&self) -> bool;
}

impl CustomSpotifyChecks for AuthCodeSpotify {
  fn has_token(&self) -> bool {
    self.get_token().lock().unwrap().is_some()
  }
}