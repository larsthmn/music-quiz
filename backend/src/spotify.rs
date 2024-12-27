use std::ops::Deref;
use std::sync::{Arc};
use tokio::sync::{Mutex, mpsc};
use std::time::{Duration};
use rspotify::AuthCodeSpotify;
use rspotify::clients::{BaseClient, OAuthClient};
use crate::{GamePreferences, GameReferences};
use chrono::prelude::*;
use rspotify::model::Id;
use crate::game::Playlist;
use futures::StreamExt;

pub async fn spotify_loop(mut rx: mpsc::Receiver<()>, preferences: Arc<Mutex<GamePreferences>>,
                          references: Arc<Mutex<GameReferences>>) {
  loop {
    // Always lock references fist to avoid deadlock!
    let r = references.lock().await;
    // Refresh token
    let mut needs_refresh = false;
    if let Some(token) = r.spotify_client.get_token().lock().await.unwrap().as_ref() {
      if Utc::now() + chrono::Duration::seconds(30) > token.expires_at.expect("Token has no expiration") {
        needs_refresh = true;
      }
    }
    if needs_refresh {
      match r.spotify_client.refresh_token().await {
        Ok(_) => log::info!("Refreshed spotify token!"),
        Err(e) => log::warn!("Error on refreshing token: {:?}", e)
      }
    }

    // Refresh playlists
    if r.spotify_client.has_token().await {
      let mut p = preferences.lock().await;
      p.playlists = r.spotify_client
        .current_user_playlists()
        .filter_map(|playlist| async move { playlist.ok() })
        .map(|playlist| Playlist { name: playlist.name, id: playlist.id.uri() })
        .collect()
        .await;
      // Select a playlist if none is selected or selected one does not exist
      if (p.selected_playlist.is_none()
        || p.playlists.iter().find(|x| x.id == p.selected_playlist.as_ref().unwrap().id).is_none())
        && p.playlists.len() > 0 {
        log::info!("set selected playlist to first one {:?}", p.playlists[0]);
        p.selected_playlist = Some(p.playlists[0].clone());
      }
      log::info!("Refreshed {} playlists", p.playlists.len());

      for playlist in &p.playlists {
        log::debug!("Playlist: {:?}", playlist);
      }
    }
    drop(r);

    match tokio::time::timeout(Duration::from_secs(20), rx.recv()).await {
      Ok(Some(())) => {
        log::debug!("Triggered playlist update");
      }
      Ok(None) => {
        log::error!("Spotify rx channel closed, playlist update trigger not possible anymore");
      }
      Err(_) => {
        log::debug!("Playlist update after 20 seconds");
      }
    }
  }
}

pub trait CustomSpotifyChecks {
  async fn has_token(&self) -> bool;
}

impl CustomSpotifyChecks for AuthCodeSpotify {
  async fn has_token(&self) -> bool {
    match self.get_token().lock().await.unwrap().deref() {
      Some(token) => !token.is_expired(),
      None => false
    }
  }
}
