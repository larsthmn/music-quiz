use std::cmp::min;
use std::ops::Deref;
use std::sync::{Arc};
use tokio::sync::{Mutex, RwLock, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use rspotify::AuthCodeSpotify;
use tokio::sync::broadcast::Sender;
use crate::game::GameError::{AnswerNotAllowed, InvalidState};
use crate::quiz::{QuizError, SongQuiz};
use ts_rs::TS;

const MAX_POINTS_CORRECT_ANSWER: i32 = 100; /// Maximum points for correct answer
const MIN_POINTS_CORRECT_ANSWER: i32 = 20;  /// Minimum points for correct answer
const TIME_FULL_POINTS_MS: u32 = 1000;  /// Time after question start in which full points are given (in time score mode)
const POINTS_TIME: [f32; 6] = [0.0, 800.0, 1300.0, 2000.0, 3000.0, 10000.0];
const POINTS_AMOUNT: [i32; 6] = [100, 100, 80, 60, 50, 20];

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct UserAnswerExposed {
  answer_id: String,
  user: String,
  #[ts(type = "number")]
  ts: u64,
}

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct AnswerExposed {
  pub text: String,
  pub id: String,
}

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct Question {
  pub text: String,
  pub answers: Vec<AnswerExposed>,
  pub correct: Option<String>,
  pub solution: Option<String>,
  pub index: i32,
  pub total_questions: u32,
}

#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
struct PlayerScoreAPI {
  player: String,
  points: i32,
  correct: u32,
  answers_given: u32,
  last_points: Option<i32>,
  last_time: Option<f32>
}

impl PlayerScoreAPI {
  pub fn new(player: String) -> PlayerScoreAPI {
    PlayerScoreAPI {
      player,
      points: 0,
      correct: 0,
      answers_given: 0,
      last_points: None,
      last_time: None,
    }
  }
}

#[derive(PartialEq, Serialize, Copy, Clone, Debug, strum_macros::Display, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub enum AppStatus {
  Shutdown,
  Ready,
  BeforeGame,
  Preparing,
  InGameAnswerPending,
  InGameWaitForNextQuestion,
  BetweenRounds,
}

// Public game management structure
#[derive(Serialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct GameState {
  status: AppStatus,
  #[ts(type = "number")]
  action_start: u64,
  #[ts(type = "number")]
  next_action: u64,
  current_question: Option<Question>,
  players: Vec<PlayerScoreAPI>,
  given_answers: Vec<UserAnswerExposed>,
  hide_answers: bool
}

// Internal game management structure
pub struct GameReferences {
  pub tx_commands: mpsc::Sender<GameCommand>,
  pub tx_spotify: mpsc::Sender<()>,
  pub spotify_client: Arc<AuthCodeSpotify>,
  pub tx_broadcast: tokio::sync::broadcast::Sender<Message>,
  pub rx_broadcast: tokio::sync::broadcast::Receiver<Message>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub enum ScoreMode {
  TimeLinear,
  TimeFunction,
  WrongFalse,
  Order,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct Playlist {
  pub name: String,
  pub id: String,
}

#[derive(Serialize, Deserialize, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct GamePreferences {
  pub scoremode: ScoreMode,
  pub playlists: Vec<Playlist>,
  pub selected_playlist: Option<Playlist>,
  pub time_to_answer: u32,
  pub time_between_answers: u32,
  pub time_before_round: u32,
  pub rounds: u32,
  pub preview_mode: bool,
  pub hide_answers: bool,
  pub ask_for_artist: bool,
  pub ask_for_title: bool
}

impl GamePreferences {
  pub fn new() -> GamePreferences {
    GamePreferences {
      scoremode: ScoreMode::WrongFalse,
      playlists: vec![],
      selected_playlist: None,
      time_to_answer: 5,
      time_before_round: 3,
      time_between_answers: 5,
      rounds: 5,
      preview_mode: false,
      hide_answers: false,
      ask_for_artist: true,
      ask_for_title: true
    }
  }
}

#[derive(PartialEq, Serialize, Clone, Debug, strum_macros::Display, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub enum GameCommand {
  StartGame,
  StopGame,
}

#[derive(Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
#[ts(export_to = "../shared/")]
pub struct AnswerFromUser {
  id: String,
  #[ts(type = "number")]
  timestamp: u64,
  user: String,
}

impl GameState {
  pub fn new() -> GameState {
    GameState {
      status: AppStatus::Shutdown,
      action_start: 0,
      next_action: 0,
      current_question: None,
      players: vec![],
      given_answers: vec![],
      hide_answers: false
    }
  }

  /// Receive an answer from a user
  pub fn give_answer(&mut self, answer: AnswerFromUser) -> Result<(), GameError> {
    if self.status != AppStatus::InGameAnswerPending {
      return Err(InvalidState(self.status));
    }
    // Check if answers already contain user somewhere
    if let Some(current_question) = &mut self.current_question {
      let user_has_selected = self.given_answers
        .iter()
        .any(|a| a.user == answer.user);

      if user_has_selected {
        return Err(AnswerNotAllowed("Already selected an answer"));
      }

      if answer.timestamp < self.action_start || answer.timestamp > self.next_action {
        return Err(AnswerNotAllowed("Timestamp of answer not in allowed range"));
      }

      // Select answer with given ID
      log::info!("Answer with ID {}", answer.id);
      let selected_answer = current_question.answers
        .iter_mut()
        .find(|a| a.id == answer.id);
      if let Some(ans) = selected_answer {
        log::info!("User {} selected {} at {}", answer.user, ans.text, answer.timestamp);
        self.given_answers.push(
          UserAnswerExposed { user: answer.user.clone(), ts: answer.timestamp, answer_id: answer.id });
      } else {
        return Err(AnswerNotAllowed("Invalid ID"));
      }
    } else {
      return Err(AnswerNotAllowed("No current question"));
    }
    Ok(())
  }
}

/// Return milliseconds from epoch.
fn get_epoch_ms() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("System time is < UNIX_EPOCH")
    .as_millis() as u64
}

/// Executes a game round:
///
/// Init => for each `question` [set question => wait for answer] => show results.
/// Preferences stay the same for the whole round.
async fn game_round(state: &Arc<RwLock<GameState>>, rx: &mut mpsc::Receiver<GameCommand>, pref: GamePreferences, spotify: Arc<AuthCodeSpotify>,
              tx_broadcast: &Sender<Message>) -> Result<(), GameError> {
  // Generate questions to be answered
  let mut s = state.write().await;
  prepare_round(&mut s);
  drop(s);

  let mut quiz = SongQuiz::new(spotify, pref.preview_mode);
  quiz.generate_questions(pref.rounds,
                          &pref.selected_playlist.as_ref().ok_or(GameError::RuntimeError("No playlist selected"))?.id,
                          pref.ask_for_artist,
                          pref.ask_for_title).await?;

  let mut s = state.write().await;
  let next_timeout = countdown_round(&mut s, &pref);
  let _ = tx_broadcast.send(s.deref().into());
  drop(s);

  // Wait for game start or stopping game
  if !wait_for_command(rx, GameCommand::StopGame, next_timeout).await {
    // Init results of this round
    for question in quiz.get_questions().clone() {
      // Set new question (state is changed first so the user sees the question before the music starts -
      // could also be done the other way around, but then the music may start when users do not see the question yet)
      // todo: start song with volume 0 to buffer, remove preview mp3s
      let mut s = state.write().await;
      let next_timeout = set_question(question.clone(), &mut s, &pref);
      let _ = tx_broadcast.send(s.deref().into());
      drop(s);
      if let Err(e) = quiz.begin_question_action(question.index as usize).await {
        log::warn!("Begin question failed with error: {:?}", e);
      }

      // Wait for users to answer or stopping game
      if wait_for_command(rx, GameCommand::StopGame, next_timeout).await {
        break;
      }

      if let Err(e) = quiz.stop_question_action(question.index as usize).await {
        log::warn!("End question failed with error: {:?}", e);
      }

      // Evaluate answers
      let mut s = state.write().await;
      let next_timeout = finish_question(&question, &mut s, &pref);
      let _ = tx_broadcast.send(s.deref().into());
      drop(s);

      // Wait for next question or stopping game
      if wait_for_command(rx, GameCommand::StopGame, next_timeout).await {
        break;
      }
    }
  }

  // show results
  let mut s = state.write().await;
  end_round(&mut s);
  let _ = tx_broadcast.send(s.deref().into());
  drop(s);

  // stop playback
  if let Err(e) =  quiz.shutdown().await {
    log::warn!("Ending round failed with error: {:?}", e);
  }

  Ok(())
}

fn prepare_round(s: &mut GameState)  {
  s.players = vec![];
  s.current_question = None;
  s.status = AppStatus::Preparing;
  s.action_start = 0;
  s.next_action = 0;
  s.given_answers = vec![];
}

/// Set the countdown where players should get ready
fn countdown_round(s: &mut GameState, pref: &GamePreferences) -> u64 {
  s.players = vec![];
  s.current_question = None;
  s.status = AppStatus::BeforeGame;
  let now = get_epoch_ms();
  s.action_start = now;
  s.next_action = now + (pref.time_before_round * 1000) as u64;
  s.given_answers = vec![];
  let next_timeout = s.next_action;
  next_timeout
}

// End the round, will display end results
fn end_round(s: &mut GameState) {
  for score in &mut s.players {
    score.last_time = None;
    score.last_points = None;
  }
  s.current_question = None;
  s.action_start = 0;
  s.next_action = 0;
  s.status = AppStatus::BetweenRounds;
}

/// Evaluate answers of users and set game state accordingly
fn finish_question(question: &Question, s: &mut GameState, pref: &GamePreferences) -> u64 {
  log::info!("Question no {} / {} finished!", question.index + 1, question.total_questions);
  s.status = AppStatus::InGameWaitForNextQuestion;
  if let Some(q) = &mut s.current_question {
    // Publish correct index
    q.correct = question.correct.clone();
    q.solution = question.solution.clone();
  }
  calc_points(s, pref);
  s.players.sort_by(|a, b| b.points.cmp(&a.points));
  let now = s.next_action;
  s.action_start = now;
  s.hide_answers = false;
  s.next_action = now + (pref.time_between_answers * 1000) as u64;
  let next_timeout = s.next_action;
  next_timeout
}

/**
Calculate the points depending on needed time and maximum time to answer
*/
fn calc_points_time(time_needed_for_answer: u64, max_answer_time: u64) -> i32 {
  if time_needed_for_answer <= TIME_FULL_POINTS_MS as u64 {
    MAX_POINTS_CORRECT_ANSWER
  } else {
    let time_after_deadzone = std::cmp::max(time_needed_for_answer - TIME_FULL_POINTS_MS as u64, 0);
    let part_needed = time_after_deadzone as f32 / (max_answer_time - TIME_FULL_POINTS_MS as u64) as f32;
    ((1.0 - part_needed).max(0.0) *
      (MAX_POINTS_CORRECT_ANSWER - MIN_POINTS_CORRECT_ANSWER) as f32 + MIN_POINTS_CORRECT_ANSWER as f32)
      .round() as i32
  }
}

/// Calculate the points for all players for the current question
fn calc_points(s: &mut GameState, pref: &GamePreferences) {
  if let Some(q) = &s.current_question {
    let given_answers = &mut s.given_answers;
    given_answers.sort_by(|a, b| a.ts.cmp(&b.ts));
    for (pos, user_ans) in given_answers.iter().enumerate() {
      // find player in results
      if !s.players.iter_mut().any(|score| score.player == user_ans.user) {
        s.players.push(PlayerScoreAPI::new(user_ans.user.clone()));
      }
      // Points need to be calculated here, because later s can't be borrowed (since score = mutable borrow)
      let time_needed_for_answer = user_ans.ts - s.action_start;
      let mut points_if_correct: i32 = match pref.scoremode {
        ScoreMode::TimeLinear => calc_points_time(time_needed_for_answer, s.next_action - s.action_start) as i32,
        ScoreMode::TimeFunction => minterpolate::linear_interpolate(time_needed_for_answer as f32, &POINTS_TIME, &POINTS_AMOUNT, false),
        ScoreMode::Order => min(MIN_POINTS_CORRECT_ANSWER, MAX_POINTS_CORRECT_ANSWER - pos as i32 * 10),
        ScoreMode::WrongFalse => MAX_POINTS_CORRECT_ANSWER
      };
      points_if_correct = std::cmp::max(MIN_POINTS_CORRECT_ANSWER,
                                        std::cmp::min(MAX_POINTS_CORRECT_ANSWER, points_if_correct));
      let score = s.players
        .iter_mut()
        .find(|score| score.player == user_ans.user)
        .expect("Player must be in Vector");
      score.answers_given += 1;
      score.last_time = Some(time_needed_for_answer as f32 / 1000.0);
      if &user_ans.answer_id == q.correct.as_ref().expect("No correct answer in calc_points") {
        score.correct += 1;
        score.last_points = Some(points_if_correct);
        score.points += points_if_correct;
      } else {
        score.last_points = Some(0);
      }
    }
  }
}

/// Set the current question to be answered
fn set_question(mut question: Question, s: &mut GameState, pref: &GamePreferences) -> u64 {
  log::info!("Question no {} / {}", question.index + 1, question.total_questions);
  question.correct = None;
  question.solution = None;
  s.current_question = Some(question);
  let now = s.next_action;
  s.action_start = now;
  s.next_action = now + (pref.time_to_answer * 1000) as u64;
  s.status = AppStatus::InGameAnswerPending;
  s.given_answers = vec![];
  s.hide_answers = if pref.hide_answers {true} else {false};
  s.next_action
}

/// Wait for a command or until some time in ms after epoch
async fn wait_for_command(rx: &mut mpsc::Receiver<GameCommand>, command: GameCommand, until: u64) -> bool {
  loop {
    let diff: i64 = (until.checked_sub(get_epoch_ms()).unwrap_or(100)) as i64;
    if diff > 0 {
      match tokio::time::timeout(Duration::from_millis((diff) as u64), rx.recv()).await {
        Ok(Some(cmd)) if cmd == command => {
          return true;
        }
        Ok(_) => {}
        Err(_) => return false
      }
    } else {
      return false;
    }
  }
}

/// Main loop for the game thread. `rx` is used to receive game commands.
pub async fn run(state: Arc<RwLock<GameState>>, mut rx: mpsc::Receiver<GameCommand>, preferences: Arc<Mutex<GamePreferences>>,
           references: Arc<Mutex<GameReferences>>) {

  let r = references.lock().await;
  let tx_broadcast = r.tx_broadcast.clone();
  drop(r);

  // Wait for start by admin?
  let mut s = state.write().await;
  s.status = AppStatus::Ready;
  s.players = vec![];
  s.action_start = 0;
  s.next_action = 0;
  s.current_question = None;
  // tx_broadcast.send(s.clone());
  drop(s);

  loop {
    // wait for game start
    wait_for_game_start(&mut rx).await;
    log::info!("Start round");

    // Get preferences
    let p_mut = preferences.lock().await;
    let pref = p_mut.clone();
    drop(p_mut);

    // Get spotify auth code
    let r_mut = references.lock().await;
    let spotify = r_mut.spotify_client.clone();
    drop(r_mut);

    // Play one round
    match game_round(&state, &mut rx, pref, spotify, &tx_broadcast).await {
      Ok(()) => log::info!("Round ended"),
      Err(e) => log::warn!("Round ended with error: {:?}", e)
    }
    // After the round the results are available to be fetched until the next round is started
  }
}

/// Wait for the Command `StartGame`
async fn wait_for_game_start(rx: &mut mpsc::Receiver<GameCommand>) {
  loop {
    if let Some(c) = rx.recv().await {
      if c == GameCommand::StartGame {
        return;
      }
    }
  }
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
  #[error("Answer not allowed: {0}")]
  AnswerNotAllowed(&'static str),

  #[error("Invalid game state {0}")]
  InvalidState(AppStatus),

  #[error("RuntimeError: {0}")]
  RuntimeError(&'static str),

  #[error("QuizError: {0}")]
  QuizError(QuizError),
}

impl From<&'static str> for GameError {
  fn from(s: &'static str) -> Self {
    GameError::RuntimeError(s)
  }
}

impl From<QuizError> for GameError {
  fn from(e: QuizError) -> Self {
    GameError::QuizError(e)
  }
}
