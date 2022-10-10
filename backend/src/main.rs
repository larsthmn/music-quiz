use axum::{Extension, response::Json, extract::Query, routing::{get, post}, extract::ws::{WebSocket, Message}};
use futures::{sink::SinkExt, stream::{StreamExt, SplitSink, SplitStream}};
use clap::Parser;
use crate::game::{AnswerFromUser, GameState, GameReferences, GameCommand, GamePreferences, ScoreMode};
use crate::spotify::{spotify_loop};
use log::{LevelFilter};
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth};
use serde::{Serialize, Deserialize};
use serde_json;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::ops::Deref;
use std::sync::{Arc, mpsc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{fs, thread};
use ts_rs::TS;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum_extra::routing::SpaRouter;
use tokio::sync::Notify;
use tokio::time::timeout;

mod game;
mod quiz;
mod spotify;

// todo: use websockets

const PREFERENCES_FILE: &'static str = "preferences.json";
const SPOTIFY_FILE: &'static str = "spotify.json";

// Setup the command line interface with clap.
#[derive(Parser, Debug)]
#[clap(name = "musicquiz-server", about = "Music Quiz Server")]
struct Opt {
  /// set the log level
  #[clap(short = 'l', long = "log", default_value = "debug")]
  log_level: String,

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

//---------------------------------------------- POST Routes -----------------------------------------------------------

async fn select_answer(Extension(state): Extension<Arc<RwLock<GameState>>>, answer: Json<AnswerFromUser>) -> Json<GameState> {
  let mut s = state.write().unwrap();
  if let Err(err) = s.give_answer(answer.deref().clone()) {
    log::warn!("Error on giving answer: {:?}", err);
  }
  Json(s.clone())
}

async fn start_game(Extension(references): Extension<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  if let Err(e) = r.tx_commands.send(GameCommand::StartGame) {
    log::warn!("Could not send game command ({:?})", e)
  }
}

async fn refresh_spotify(Extension(references): Extension<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  if let Err(e) = r.tx_spotify.send(()) {
    log::warn!("Could not send spotify wakeup ({:?})", e)
  }
}

async fn authorize_spotify(Extension(references): Extension<Arc<Mutex<GameReferences>>>, Query(params): Query<HashMap<String, String>>) {
  let mut r = references.lock().unwrap();
  let code = params.get("code");
  log::info!("received spotify auth code: {:?}", code);
  if let Some(c) = code {
    let result = r.spotify_client.request_token(c.as_str());
    match result {
      Ok(_) => log::info!("Got auth token!"),
      Err(e) => log::warn!("Could not get auth token {:?}", e)
    }
  }
}

#[derive(Deserialize)]
struct PreferenceParams {
  scoremode: Option<ScoreMode>,
  playlist: Option<String>,
  time_to_answer: Option<u32>,
  time_between_answers: Option<u32>,
  time_before_round: Option<u32>,
  rounds: Option<u32>,
  preview_mode: Option<bool>,
  hide_answers: Option<bool>,
  ask_for_artist: Option<bool>,
  ask_for_title: Option<bool>,
}

async fn set_preference(Extension(preferences): Extension<Arc<Mutex<GamePreferences>>>, params: Query<PreferenceParams>)
                        -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  if let Some(sm) = params.scoremode {
    log::info!("set scoremode to {:?}", sm);
    p.scoremode = sm;
  }
  if let Some(id) = &params.playlist {
    if let Some(selected_playlist) = p.playlists.iter().find(|x| x.id == *id) {
      log::info!("set playlist to {:?}", selected_playlist);
      p.selected_playlist = Some(selected_playlist.clone());
    }
  }
  if let Some(t) = params.time_to_answer {
    log::info!("set time_to_answer to {}", t);
    p.time_to_answer = t;
  }
  if let Some(t) = params.time_between_answers {
    log::info!("set time_between_answers to {}", t);
    p.time_between_answers = t;
  }
  if let Some(t) = params.time_before_round {
    log::info!("set time_before_round to {}", t);
    p.time_before_round = t;
  }
  if let Some(r) = params.rounds {
    log::info!("set rounds to {}", r);
    p.rounds = r;
  }
  if let Some(m) = params.preview_mode {
    log::info!("set preview_mode to {}", m);
    p.preview_mode = m;
  }
  if let Some(m) = params.hide_answers {
    log::info!("set hide_answers to {}", m);
    p.hide_answers = m;
  }
  if let Some(a) = params.ask_for_title {
    p.ask_for_title = if p.ask_for_artist { a } else { true };
    log::info!("set ask_for_title to {}", p.ask_for_title);
  }
  if let Some(a) = params.ask_for_artist {
    p.ask_for_artist = if p.ask_for_title { a } else { true };
    log::info!("set ask_for_title to {}", p.ask_for_artist);
  }
  let new_preferences = p.clone();
  drop(p);
  save_preferences(&new_preferences, PREFERENCES_FILE);
  Json(new_preferences)
}

fn save_preferences(new_preferences: &GamePreferences, to: &str) {
  match fs::File::create(to) {
    Ok(file) => match serde_json::to_writer_pretty::<fs::File, GamePreferences>(file, &new_preferences) {
      Ok(_) => log::info!("Saved preferences to file"),
      Err(e) => log::warn!("Could not save preferences to file ({:?})", e)
    },
    Err(e) => log::warn!("Could not open file to write: {:?}", e)
  }
}

async fn set_preferences(Extension(preferences): Extension<Arc<Mutex<GamePreferences>>>, received: Json<GamePreferences>)
                         -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  *p = received.deref().clone();
  let new_preferences = p.clone();
  drop(p);
  save_preferences(&new_preferences, PREFERENCES_FILE);
  Json(new_preferences)
}

async fn stop_game(Extension(references): Extension<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  match r.tx_commands.send(GameCommand::StopGame) {
    Err(e) => log::warn!("Game could not be stopped: {}", e),
    Ok(_) => log::info!("Stopped game")
  }
}

//----------------------------------------------- GET Routes -----------------------------------------------------------

// #[get("/get_state")]
async fn get_state(Extension(state): Extension<Arc<RwLock<GameState>>>) -> Json<GameState> {
  let s = state.read().unwrap();
  Json(s.clone())
}

async fn get_preferences(Extension(preferences): Extension<Arc<Mutex<GamePreferences>>>)
                         -> Json<GamePreferences> {
  let p = preferences.lock().unwrap();
  Json(p.clone())
}

#[derive(Serialize)]
struct TimeAnswer {
  diff_receive: i64,
  ts: u64,
}

async fn get_time(Query(params): Query<HashMap<String, i64>>) -> Json<TimeAnswer> {
  let now_ms = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("System time is < UNIX_EPOCH")
    .as_millis() as u64;
  let now = params.get("now");
  match now {
    Some(&now) => Json(TimeAnswer { diff_receive: now as i64 - now_ms as i64, ts: now_ms }),
    None => Json(TimeAnswer { diff_receive: 0, ts: now_ms })
  }
}

#[derive(Deserialize, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
struct SpotifyPrefs {
  scopes: Vec<String>,
  redirect_uri: String,
  client_id: String,
  client_secret: String,
}

impl SpotifyPrefs {
  fn new() -> SpotifyPrefs {
    SpotifyPrefs {
      scopes: vec!["user-modify-playback-state".to_string(),
                   "user-read-playback-state".to_string(),
                   "user-read-currently-playing".to_string(),
                   "playlist-read-collaborative".to_string(),
                   "playlist-read-private".to_string(),
                   "app-remote-control".to_string(),
                   "streaming".to_string(),
                   "user-read-email".to_string(),
                   "user-read-private".to_string()],
      redirect_uri: "http://localhost:80/redirect".to_string(),
      client_id: "d071021f312148b38eaa0243f11a52c8".to_string(),
      client_secret: "123456789".to_string(),
    }
  }
}


async fn ws_handler(ws: WebSocketUpgrade, Extension(state): Extension<Arc<RwLock<GameState>>>,
                    Extension(references): Extension<Arc<Mutex<GameReferences>>>) -> impl IntoResponse {
  let r = references.lock().unwrap();
  let notify1 = r.notify.clone();
  let notify2 = r.notify.clone();
  drop(r);
  ws.on_upgrade(|socket| async move {
    log::debug!("Client connected");
    let (sender, receiver) = socket.split();

    tokio::spawn(read_socket(receiver, state.clone(), notify1));
    tokio::spawn(write_socket(sender, state, notify2));
  })
}

async fn read_socket(mut receiver: SplitStream<WebSocket>, state: Arc<RwLock<GameState>>, notify: Arc<Notify>) {
  while let Some(result) = receiver.next().await {
    // Only thing that can be received is an answer (currently)
    match result {
      Ok(msg) => if let Ok(answer) = serde_json::from_str::<AnswerFromUser>(msg.into_text().unwrap().as_str()) {
        let mut s = state.write().unwrap();
        if let Err(err) = s.give_answer(answer) {
          log::warn!("Error on giving answer: {:?}", err);
        }
        drop(s);
        notify.notify_waiters();
      },
      Err(err) => {
        // client disconnected
        log::debug!("Client disconnected with error {}", err);
        return;
      }
    }
  };
  // client disconnected
  log::debug!("Client disconnected");
}

async fn write_socket(mut sender: SplitSink<WebSocket, Message>, state: Arc<RwLock<GameState>>, notify: Arc<Notify>) {
  let mut is_connected = true;
  while is_connected
  {
    let json;
    {
      // Needs its own scope for the mutex
      let s = state.read().unwrap();
      json = serde_json::to_string(s.deref());
    }
    match json {
      Ok(str) => {
        let result = sender.send(Message::Text(str)).await;
        is_connected = result.is_ok();
      }
      Err(..) => log::error!("Could not make JSON from GameState"),
    }

    // Wait for notification or timeout (don't care if it's a timeout or notified => ignore result)
    let _ = timeout(Duration::from_millis(5000), notify.notified()).await;
    log::info!("Send state");
  }
}

#[tokio::main]
async fn main() {
  SimpleLogger::new()
    .with_level(LevelFilter::Info)
    .with_module_level("_", LevelFilter::Warn)
    .init()
    .unwrap();

  let opt = Opt::parse();

  // Internal objects
  let (tx_cmd, rx_cmd) = mpsc::channel::<GameCommand>();
  let (tx_spotify, rx_spotify) = mpsc::channel::<()>(); // Nothing to be transferred, just to wakeup
  let notify = Arc::new(Notify::new());

  // Read spotify preferences and create clients
  let mut spotify_prefs = SpotifyPrefs::new();
  if let Ok(file) = fs::File::open(SPOTIFY_FILE) {
    if let Ok(s) = serde_json::from_reader::<fs::File, SpotifyPrefs>(file) {
      spotify_prefs = s;
      log::info!("Successfully read file {}", SPOTIFY_FILE);
    } else {
      log::warn!("File {} not in expected format, using default", SPOTIFY_FILE);
    }
  } else {
    log::warn!("Did not find {}, using default", SPOTIFY_FILE);
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
    GameReferences { tx_commands: tx_cmd, tx_spotify, spotify_client, notify }));
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

