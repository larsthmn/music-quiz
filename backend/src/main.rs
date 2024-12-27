use std::{fs};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc};
use tokio::sync::{Mutex, RwLock, mpsc};
use axum::{Extension, extract::ws::Message, routing::{get, post}};
use clap::Parser;
use log::LevelFilter;
use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth};
use rspotify::clients::{BaseClient, OAuthClient};
use serde_json;
use simple_logger::SimpleLogger;
use tower_http::services::ServeDir;
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
  #[clap(short = 'l', long = "log", default_value = "INFO")]
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
  let opt = Opt::parse();

  SimpleLogger::new()
    .with_level(LevelFilter::Warn)
    .with_module_level("music_quiz", LevelFilter::from_str(opt.log_level.as_str()).unwrap())
    .init()
    .unwrap();

  // Internal objects
  // channel to send GameCommands like start and stop to the game thread
  let (tx_cmd, rx_cmd) = mpsc::channel::<GameCommand>(32);
  // channel to wake up spotify thread
  let (tx_spotify, rx_spotify) = mpsc::channel::<()>(32);
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
  let spotify_client = AuthCodeSpotify::with_config(creds,
                                                        OAuth {
                                                          scopes: spotify_prefs.scopes.into_iter().collect(),
                                                          redirect_uri,
                                                          ..Default::default()
                                                        },
                                                        Config { token_cached: true, ..Default::default() });
  match spotify_client.read_token_cache(true).await {
    Ok(token) => {
      *spotify_client.get_token().lock().await.unwrap() = token;
      match spotify_client.refresh_token().await {
        Ok(()) => log::info!("Refreshed token"),
        Err(e) => log::warn!("Could not refresh token on start: {:?}", e)
      }
    }
    Err(e) => log::warn!("Could not load token: {:?}", e)
  }

  // Shared objects
  let spotify_arc = Arc::new(spotify_client);
  let references = Arc::new(Mutex::new(
    GameReferences { tx_commands: tx_cmd, tx_spotify, spotify_client: spotify_arc, tx_broadcast, rx_broadcast }));
  let mut game_pref = GamePreferences::new();
  if let Ok(file) = fs::File::open(PREFERENCES_FILE) {
    if let Ok(p) = serde_json::from_reader::<fs::File, GamePreferences>(file) {
      game_pref = p;
    }
  }
  let preferences = Arc::new(Mutex::new(game_pref));
  let gamestate = Arc::new(RwLock::new(GameState::new()));

  let g = gamestate.clone();
  let p = preferences.clone();
  let r = references.clone();
  let handle_gametask = tokio::spawn(async move { game::run(g, rx_cmd, p, r).await });

  // Spawn spotify task
  let p = preferences.clone();
  let r = references.clone();
  let handle_spotifytask = tokio::spawn(async move { spotify_loop(rx_spotify, p, r).await });

  let static_files_service = ServeDir::new("files");

  // Start HTTP interface
  // SPA Router serves all files at /files, GET / gives /files/index.html
  // In frontend/package.json the homepage is configured as files which makes all files to be expected in /files
  let app = axum::Router::new()
    .nest_service("/files", static_files_service)
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
    .layer(Extension(gamestate))  // todo: with_state instead?
    .layer(Extension(references))
    .layer(Extension(preferences));

  let addr = SocketAddr::from((opt.addr.parse::<Ipv4Addr>().unwrap(), opt.port));
  log::info!("Starting server at {}", addr);

  let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
  axum::serve(listener, app)
    .await
    .unwrap();

  handle_gametask.await.unwrap();
  handle_spotifytask.await.unwrap();

  log::info!("Goodbye.");
}

