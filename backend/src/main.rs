use std::{fs, thread};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, mpsc, Mutex, RwLock};
use axum::{Extension, extract::ws::Message, routing::{get, post}};
use axum_extra::routing::SpaRouter;
use clap::Parser;
use log::LevelFilter;
use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth};
use rspotify::clients::{BaseClient, OAuthClient};
use serde_json;
use simple_logger::SimpleLogger;

use crate::communication::*;
use crate::game::{GameCommand, GamePreferences, GameReferences, GameState};
use crate::spotify::spotify_loop;

mod game;
mod quiz;
mod spotify;
mod communication;

const PREFERENCES_FILE: &'static str = "preferences.json";

// Setup the command line interface with clap.
#[derive(Parser, Debug)]
#[clap(name = "musicquiz-server", about = "Music Quiz Server")]
struct Opt {
  /// set the log level
  #[clap(short = 'l', long = "log", default_value = "debug")]
  log_level: String,

  /// set the spotify config file
  #[clap(short = 's', long = "spotify", default_value = "spotify.json")]
  spotify_json: String,

  /// set the listen addr
  #[clap(short = 'a', long = "addr", default_value = "0.0.0.0")]
  addr: String,

  /// set the listen port
  #[clap(short = 'p', long = "port", default_value = "80")]
  port: u16,

  /// set the directory where static files are to be found
  #[clap(long = "static-dir", default_value = "../dist")]
  static_dir: String,
}


#[tokio::main]
async fn main() {
  SimpleLogger::new()
    .with_level(LevelFilter::Warn)
    .with_module_level("music_quiz", LevelFilter::Debug)
    .init()
    .unwrap();

  let opt = Opt::parse();

  // Internal objects
  // channel to send GameCommands like start and stop to the game thread
  let (tx_cmd, rx_cmd) = mpsc::channel::<GameCommand>();
  // channel to wake up spotify thread
  let (tx_spotify, rx_spotify) = mpsc::channel::<()>();
  // channel for broadcast messages (mainly state for all when one gives an answer)
  let (tx_broadcast, rx_broadcast) = tokio::sync::broadcast::channel::<Message>(8);

  // Read spotify preferences and create clients
  let mut spotify_prefs = SpotifyPrefs::new();
  if let Ok(file) = fs::File::open(&opt.spotify_json) {
    if let Ok(s) = serde_json::from_reader::<fs::File, SpotifyPrefs>(file) {
      spotify_prefs = s;
      log::info!("Successfully read file {}", &opt.spotify_json);
    } else {
      log::warn!("File {} not in expected format, using default", &opt.spotify_json);
    }
  } else {
    log::warn!("Did not find {}, using default", &opt.spotify_json);
  }
  let creds = Credentials { id: spotify_prefs.client_id, secret: Some(spotify_prefs.client_secret) };
  let redirect_uri = spotify_prefs.redirect_uri;
  let mut spotify_client = AuthCodeSpotify::with_config(creds,
                                                        OAuth {
                                                          scopes: spotify_prefs.scopes.into_iter().collect(),
                                                          redirect_uri,
                                                          ..Default::default()
                                                        },
                                                        Config { token_cached: true, ..Default::default() });
  match spotify_client.read_token_cache(true) {
    Ok(token) => {
      *spotify_client.get_token().lock().unwrap() = token;
      match spotify_client.refresh_token() {
        Ok(()) => log::info!("Refreshed token"),
        Err(e) => log::warn!("Could not refresh token on start: {:?}", e)
      }
    }
    Err(e) => log::warn!("Could not load token: {:?}", e)
  }

  // Shared objects
  let references = Arc::new(Mutex::new(
    GameReferences { tx_commands: tx_cmd, tx_spotify, spotify_client, tx_broadcast, rx_broadcast }));
  let mut game_pref = GamePreferences::new();
  if let Ok(file) = fs::File::open(PREFERENCES_FILE) {
    if let Ok(p) = serde_json::from_reader::<fs::File, GamePreferences>(file) {
      game_pref = p;
    }
  }
  let preferences = Arc::new(Mutex::new(game_pref));
  let gamestate = Arc::new(RwLock::new(GameState::new()));

  // Spawn Game thread
  let g = gamestate.clone();
  let p = preferences.clone();
  let r = references.clone();
  let handle_gamethread = thread::spawn(move || { game::run(g, rx_cmd, p, r) });

  // Spawn spotify thread
  let p = preferences.clone();
  let r = references.clone();
  let handle_spotifythread = thread::spawn(move || { spotify_loop(rx_spotify, p, r) });

  // Start HTTP interface
  // SPA Router serves all files at /files, GET / gives /files/index.html
  // In frontend/package.json the homepage is configured as files which makes all files to be expected in /files
  let app = axum::Router::new()
    .merge(SpaRouter::new("/files", "files"))
    .route("/get_state", get(get_state))
    .route("/get_time", get(get_time))
    .route("/get_preferences", get(get_preferences))
    .route("/stop_game", post(stop_game))
    .route("/start_game", post(start_game))
    .route("/press_button", post(select_answer))
    .route("/set_preferences", post(set_preferences))
    .route("/set", post(set_preference))
    .route("/authorize_spotify", post(authorize_spotify))
    .route("/refresh_spotify", post(refresh_spotify))
    .route("/ws", get(ws_handler))
    .layer(Extension(gamestate))
    .layer(Extension(references))
    .layer(Extension(preferences));

  let addr = SocketAddr::from((opt.addr.parse::<Ipv4Addr>().unwrap(), opt.port));
  log::info!("Starting server at {}", addr);

  axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();

  handle_gamethread.join().unwrap();
  handle_spotifythread.join().unwrap();

  log::info!("Goodbye.");
}

