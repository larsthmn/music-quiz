use std::fs;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{Extension, extract::Query, extract::ws::{Message, WebSocket}, response::Json};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use futures::{sink::SinkExt, stream::{SplitSink, SplitStream, StreamExt}};
use rspotify::clients::OAuthClient;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::select;
use tokio::sync::broadcast::{Receiver, Sender};
use ts_rs::TS;

use crate::game::{AnswerFromUser, GameCommand, GamePreferences, GameReferences, GameState, ScoreMode};

//---------------------------------------------- POST Routes -----------------------------------------------------------

pub async fn select_answer(Extension(state): Extension<Arc<RwLock<GameState>>>, answer: Json<AnswerFromUser>) -> Json<GameState> {
  let mut s = state.write().unwrap();
  if let Err(err) = s.give_answer(answer.deref().clone()) {
    log::warn!("Error on giving answer: {:?}", err);
  }
  Json(s.clone())
}

pub async fn start_game(Extension(references): Extension<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  if let Err(e) = r.tx_commands.send(GameCommand::StartGame) {
    log::warn!("Could not send game command ({:?})", e)
  }
}

pub async fn refresh_spotify(Extension(references): Extension<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  if let Err(e) = r.tx_spotify.send(()) {
    log::warn!("Could not send spotify wakeup ({:?})", e)
  }
}

pub async fn authorize_spotify(Extension(references): Extension<Arc<Mutex<GameReferences>>>, Query(params): Query<HashMap<String, String>>) {
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
pub struct PreferenceParams {
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

pub async fn set_preference(Extension(preferences): Extension<Arc<Mutex<GamePreferences>>>, params: Query<PreferenceParams>)
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
  save_preferences(&new_preferences, crate::PREFERENCES_FILE);
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

pub async fn set_preferences(Extension(preferences): Extension<Arc<Mutex<GamePreferences>>>, received: Json<GamePreferences>)
                             -> Json<GamePreferences> {
  let mut p = preferences.lock().unwrap();
  *p = received.deref().clone();
  let new_preferences = p.clone();
  drop(p);
  save_preferences(&new_preferences, crate::PREFERENCES_FILE);
  Json(new_preferences)
}

pub async fn stop_game(Extension(references): Extension<Arc<Mutex<GameReferences>>>) {
  let r = references.lock().unwrap();
  match r.tx_commands.send(GameCommand::StopGame) {
    Err(e) => log::warn!("Game could not be stopped: {}", e),
    Ok(_) => log::info!("Stopped game")
  }
}

//----------------------------------------------- GET Routes -----------------------------------------------------------

pub async fn get_state(Extension(state): Extension<Arc<RwLock<GameState>>>) -> Json<GameState> {
  let s = state.read().unwrap();
  Json(s.clone())
}

pub async fn get_preferences(Extension(preferences): Extension<Arc<Mutex<GamePreferences>>>)
                             -> Json<GamePreferences> {
  let p = preferences.lock().unwrap();
  Json(p.clone())
}

#[derive(Serialize, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct TimeAnswer {
  #[ts(type = "number")]
  diff_receive: i64,
  #[ts(type = "number")]
  ts: u64,
  #[ts(type = "number")]
  ts_received: u64
}

pub async fn get_time(Query(params): Query<HashMap<String, u64>>) -> Json<TimeAnswer> {
  let now_ms = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("System time is < UNIX_EPOCH")
    .as_millis() as u64;
  let now = params.get("now");
  match now {
    Some(&now) => Json(TimeAnswer { diff_receive: now as i64 - now_ms as i64, ts: now_ms, ts_received: now }),
    None => Json(TimeAnswer { diff_receive: 0, ts: now_ms, ts_received: 0 })
  }
}

#[derive(Deserialize, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct SpotifyPrefs {
  pub scopes: Vec<String>,
  pub redirect_uri: String,
  pub client_id: String,
  pub client_secret: String,
}

impl SpotifyPrefs {
  pub fn new() -> SpotifyPrefs {
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

//----------------------------------------------- WebSockets -----------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct TimeRequest {
  #[ts(type = "number")]
  now: u64,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub enum DataType {
  Answer,
  GameState,
  Time,
}

#[derive(Deserialize, Serialize, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct WebSocketMessage {
  message_type: DataType,
  data: String,
}

pub async fn ws_handler(ws: WebSocketUpgrade, Extension(state): Extension<Arc<RwLock<GameState>>>,
                        Extension(references): Extension<Arc<Mutex<GameReferences>>>) -> impl IntoResponse {
  let r = references.lock().unwrap();
  let tx_broadcast = r.tx_broadcast.clone();
  let rx_broadcast = r.tx_broadcast.subscribe();
  drop(r);
  ws.on_upgrade(|socket| async move {
    log::debug!("Client connected");
    let (sender, receiver) = socket.split();
    let (tx, rx) = tokio::sync::mpsc::channel::<Message>(8);

    tokio::spawn(read_socket(receiver, state.clone(), tx_broadcast, tx));
    tokio::spawn(write_socket(sender, state, rx_broadcast, rx));
  })
}

async fn read_socket(mut receiver: SplitStream<WebSocket>, state: Arc<RwLock<GameState>>, tx_broadcast: Sender<Message>,
                     tx_single: tokio::sync::mpsc::Sender<Message>) {
  while let Some(result) = receiver.next().await {
    // Only thing that can be received is an answer (currently)
    match result {
      Ok(ws_msg) => {
        if let Ok(msg) = serde_json::from_str::<WebSocketMessage>(ws_msg.into_text().unwrap().as_str()) {
          match msg.message_type {
            DataType::Answer => {
              // User clicked an answer, select his guess
              if let Ok(answer) = serde_json::from_str::<AnswerFromUser>(msg.data.as_str()) {
                let mut s = state.write().unwrap();
                if let Err(err) = s.give_answer(answer) {
                  log::warn!("Error on giving answer: {:?}", err);
                } else {
                  // State changes when answer is given, send broadcast with new state
                  if let Err(e) = tx_broadcast.send(s.deref().into()) {
                    log::warn!("Error on sending broadcast {:?}", e);
                  }
                }
              }
            }

            DataType::Time => {
              // User sent his timestamp, answer with diff
              match serde_json::from_str::<TimeRequest>(msg.data.as_str()) {
                Ok(request) => {
                  // State changes when answer is given, send broadcast with new state
                  let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("System time is < UNIX_EPOCH")
                    .as_millis() as u64;
                  let answer = TimeAnswer { diff_receive: request.now as i64 - now_ms as i64, ts: now_ms , ts_received: request.now };
                  if let Err(e) = tx_single.send((&answer).into()).await {
                    log::warn!("Error on sending time answer {:?}", e);
                  }
                }
                Err(e) => log::warn!("Invalid time request: {:?}", e)
              }
            }

            _ => log::warn!("Unknowm data type {:?}", msg.message_type)
          }
        }
      }
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

async fn write_socket(mut sender: SplitSink<WebSocket, Message>, state: Arc<RwLock<GameState>>,
                      mut rx_broadcast: Receiver<Message>, mut rx: tokio::sync::mpsc::Receiver<Message>) {
  // initial package after connection to give client the current state as fast as possible
  log::debug!("Send first");
  // Make message from state and send
  let msg : Message = (||
    {
      let s = state.read().unwrap();
      Message::from(s.deref())
    })();
  if sender.send(msg).await.is_err() {
    return;
  }

  const MIN_STATE_PERIOD : u64 = 500;

  // Timer to ensure that the state is sent after some time without broadcasts
  let timer = tokio::time::sleep(Duration::from_millis(MIN_STATE_PERIOD));
  tokio::pin!(timer);
  loop
  {
    select! {
      // Send state periodically
      _ = &mut timer => {
        log::debug!("Send interval");
        timer.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(MIN_STATE_PERIOD));
        // Make message from state and send
        let msg : Message = (||
        {
          let s = state.read().unwrap();
          Message::from(s.deref())
        })();
        if sender.send(msg).await.is_err() {
          return;
        }
      },

      // Send received messages
      res = rx_broadcast.recv() => {
        log::debug!("Send broadcast");
        timer.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(MIN_STATE_PERIOD));
        match res {
          Ok(msg) => if sender.send(msg).await.is_err() {
            // Send failed
            log::debug!("Send broadcast failed, closing");
            return;
          },
          Err(_) => { // Channel has been closed
            log::debug!("Broadcast channel has been closed");
            return;
          }
        }
      },
      res = rx.recv() => {
        log::debug!("Send single msg");
        match res {
          Some(msg) => if sender.send(msg).await.is_err() {
              log::debug!("Send single failed, closing");
              return; // Send failed
            },
          None => {
            // Channel has been closed
            log::debug!("Single channel closed, closing");
            return;
          }
        }
      },
    }
  }
}

impl From<&GameState> for Message {
  fn from(state: &GameState) -> Self {
    let state_json = serde_json::to_string::<GameState>(&state).unwrap();
    let ws_msg = WebSocketMessage { message_type: DataType::GameState, data: state_json };
    Message::Text(serde_json::to_string::<WebSocketMessage>(&ws_msg).unwrap())
  }
}

impl From<&TimeAnswer> for Message {
  fn from(ans: &TimeAnswer) -> Self {
    let time_answer_json = serde_json::to_string::<TimeAnswer>(&ans).unwrap();
    let ws_msg = WebSocketMessage { message_type: DataType::Time, data: time_answer_json };
    Message::Text(serde_json::to_string::<WebSocketMessage>(&ws_msg).unwrap())
  }
}