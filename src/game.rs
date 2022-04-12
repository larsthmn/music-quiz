use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rocket::serde::{Deserialize, Serialize};
use crate::game::GameError::{AnswerNotAllowed, InvalidState};
use crate::Ready;

// todo: as param, configure by admin
const TIME_BETWEEN_ROUNDS: Duration = Duration::from_secs(5);
const TIME_TO_ANSWER: Duration = Duration::from_secs(5);
const TIME_BETWEEN_ANSWERS: Duration = Duration::from_secs(3);
const TIME_BEFORE_ROUND: Duration = Duration::from_secs(3);

#[derive(Serialize, Clone)]
pub struct UserAnswer {
    pub name: String,
    pub ts: u64,
}

#[derive(Serialize, Clone)]
pub struct Answer {
    pub text: String,
    pub id: u64,
    pub given_answers: Vec<UserAnswer>,
}

#[derive(Serialize, Clone)]
pub struct Question {
    pub text: String,
    pub answers: Vec<Answer>,
    pub correct: u64,
    pub index: i32,
    pub total_questions: i32,
}

#[derive(Serialize, Clone)]
pub struct PlayerScore {
    pub player: String,
    pub points: i32,
    pub correct: u32,
    pub answers_given: u32,
}

impl PlayerScore {
    pub fn new(player: String) -> PlayerScore {
        PlayerScore {
            player,
            points: 0,
            correct: 0,
            answers_given: 0,
        }
    }
}

#[derive(PartialEq, Serialize, Copy, Clone, Debug, strum_macros::Display)]
pub enum AppStatus {
    Shutdown,
    Ready,
    BeforeGame,
    InGameAnswerPending,
    InGameWaitForNextQuestion,
    BetweenRounds,
}

// Public game management structure
#[derive(Serialize, Clone)]
pub struct GameState {
    pub status: AppStatus,
    pub action_start: u64,
    pub next_action: u64,
    pub current_question: Option<Question>,
    pub results: Vec<PlayerScore>,
}

// Private game management structure
pub struct GameReferences {
    pub spotify_handle: u64,
    // placeholder
    pub tx_commands: mpsc::Sender<GameCommand>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, strum_macros::Display, FromFormField)]
pub enum ScoreMode {
    Time,
    WrongFalse,
    Order,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GamePreferences {
    pub scoremode: ScoreMode,
}

impl GamePreferences {
    pub fn new() -> GamePreferences {
        GamePreferences { scoremode: ScoreMode::WrongFalse }
    }
}

#[derive(PartialEq, Serialize, Clone, Debug, strum_macros::Display)]
pub enum GameCommand {
    StartGame(String),
    StopGame,
}

#[derive(Serialize, Deserialize)]
pub struct AnswerFromUser {
    id: u64,
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
            results: vec![],
        }
    }

    /// Receive an answer from a user
    pub fn give_answer(&mut self, answer: AnswerFromUser) -> Result<(), GameError> {
        if self.status != AppStatus::InGameAnswerPending {
            return Err(InvalidState(self.status));
        }
        // Check if answers already contain user somewhere
        if let Some(current_question) = &mut self.current_question {
            let user_has_selected = current_question.answers
                .iter()
                .any(|a| a.given_answers.iter().any(|u| u.name == answer.user));

            if user_has_selected {
                return Err(AnswerNotAllowed("Already selected an answer"));
            }

            if answer.timestamp < self.action_start || answer.timestamp > self.next_action {
                return Err(AnswerNotAllowed("Timestamp of answer not in allowed range"));
            }

            // Select answer with given ID
            let selected_answer = current_question.answers
                .iter_mut()
                .find(|a| a.id == answer.id);
            if let Some(ans) = selected_answer {
                println!("User {} selected {} at {}", answer.user, ans.text, answer.timestamp);
                ans.given_answers.push(UserAnswer{name: answer.user.clone(), ts: answer.timestamp});
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
/// Init => for each `question` [set question => wait for answer] => show results
fn game_round(state: &Arc<Mutex<GameState>>, questions: Vec<Question>, rx: &Receiver<GameCommand>) {
    let mut s = state.lock().unwrap();
    let next_timeout = prepare_round(&mut s);
    drop(s);

    // Wait for game start or stopping game
    if !wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
        // Init results of this round
        for question in questions {
            // Set new question
            let mut s = state.lock().unwrap();
            let next_timeout = set_question(question.clone(), &mut s);
            drop(s);

            // Wait for users to answer or stopping game
            if wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
                break;
            }

            // Evaluate answers
            let mut s = state.lock().unwrap();
            let next_timeout = finish_question(&question, &mut s);
            drop(s);

            // Wait for next question or stopping game
            if wait_for_command(&rx, GameCommand::StopGame, next_timeout) {
                break;
            }
        }
    }

    // show results
    let mut s = state.lock().unwrap();
    end_round(&mut s);
    drop(s);
}

fn prepare_round(s: &mut MutexGuard<GameState>) -> u64 {
    s.results = vec![];
    s.current_question = None;
    s.status = AppStatus::BeforeGame;
    let now = get_epoch_ms();
    s.action_start = now;
    s.next_action = now + TIME_BEFORE_ROUND.as_millis() as u64;
    let next_timeout = s.next_action;
    next_timeout
}

fn end_round(s: &mut MutexGuard<GameState>) {
    s.current_question = None;
    s.action_start = 0;
    s.next_action = 0;
    s.status = AppStatus::BetweenRounds;
}

/// Evaluate answers of users and set game state accordingly
fn finish_question(question: &Question, s: &mut MutexGuard<GameState>) -> u64 {
    println!("Question no {} / {} finished!", question.index + 1, question.total_questions);
    s.status = AppStatus::InGameWaitForNextQuestion;
    if let Some(q) = &mut s.current_question {
        // Publish correct index
        q.correct = question.correct;
    }
    if let Some(q) = s.current_question.clone() { // todo: does it work without clone somehow?
        for ans in &q.answers {

            for user_ans in &ans.given_answers {
                // find player in results
                if !s.results.iter_mut().any(|score| score.player == user_ans.name) {
                    s.results.push(PlayerScore::new(user_ans.name.clone()));
                }
                // Points need to be calculated here, because later s can't be borrowed (since score = mutable borrow)
                let points_if_correct = ((1.0 - (user_ans.ts - s.action_start) as f32 /
                  (s.next_action - s.action_start) as f32) * 100.0).round() as i32;
                let score = s.results
                  .iter_mut()
                  .find(|score| score.player == user_ans.name)
                  .expect("Player must be in Vector");
                score.answers_given += 1;
                if ans.id == q.correct {
                    score.correct += 1;
                    score.points += points_if_correct;
                }
            }
        }
    }
    s.results.sort_by(|a, b| b.points.cmp(&a.points));
    let now = s.next_action;
    s.action_start = now;
    s.next_action = now + TIME_BETWEEN_ANSWERS.as_millis() as u64;
    let next_timeout = s.next_action;
    next_timeout
}

/// Set the current question to be answered
fn set_question(mut question: Question, s: &mut GameState) -> u64 {
    println!("Question no {} / {}", question.index + 1, question.total_questions);
    question.correct = 0;
    s.current_question = Some(question);
    let now = s.next_action;
    s.action_start = now;
    s.next_action = now + TIME_TO_ANSWER.as_millis() as u64;
    s.status = AppStatus::InGameAnswerPending;
    s.next_action
}

/// Wait for a command or until some time in ms after epoch
fn wait_for_command(rx: &Receiver<GameCommand>, command: GameCommand, until: u64) -> bool {
    loop {
        let diff: i64 = (until - get_epoch_ms()) as i64;
        if diff > 0 {
            match rx.recv_timeout(Duration::from_millis((diff) as u64)) {
                Ok(cmd) if cmd == command => {
                    return true;
                },
                Ok(_) => {}
                Err(_) => return false
            }
        } else {
            return false;
        }
    }
}

/// Main loop for the game thread. `rx` is used to receive game commands.
pub fn run(state: Arc<Mutex<GameState>>, rx: mpsc::Receiver<GameCommand>) {

    // Wait for start by admin?
    let mut s = state.lock().unwrap();
    s.status = Ready;
    s.results = vec![];
    let now = get_epoch_ms();
    s.action_start = now;
    s.next_action = now + TIME_BETWEEN_ROUNDS.as_millis() as u64;
    s.current_question = None;
    drop(s);

    loop {
        // wait for game start
        println!("Start round");
        wait_for_game_start(&rx);

        // generate questions, connect to spotify, prepare everything for the round
        let questions = generate_questions();

        game_round(&state, questions, &rx);
        // After the round the results are available to be fetched until the next round is started

        println!("Round ended");
    }
}

/// Wait for the Command `StartGame`
fn wait_for_game_start(rx: &Receiver<GameCommand>) {
    loop {
        if let Ok(c) = rx.recv() {
            if let GameCommand::StartGame(playlist) = c {
                println!("Started game by admin with playlist {}", playlist);
                break;
            }
        }
    }
}

/// Placeholder for generating questions from preferences and playlists
fn generate_questions() -> Vec<Question> {
    vec![
        Question {
            text: "Frage 1".to_string(),
            answers: vec![Answer { text: "A11 richtig".to_string(), id: 11, given_answers: vec![] },
                          Answer { text: "A12 falsch".to_string(), id: 12, given_answers: vec![] },
                          Answer { text: "A13 falsch".to_string(), id: 13, given_answers: vec![] },
                          Answer { text: "A14 falsch".to_string(), id: 14, given_answers: vec![] }],
            correct: 11,
            index: 0,
            total_questions: 3,
        },
        Question {
            text: "Frage 2".to_string(),
            answers: vec![Answer { text: "A21 falsch".to_string(), id: 21, given_answers: vec![] },
                          Answer { text: "A22 richtig".to_string(), id: 22, given_answers: vec![] },
                          Answer { text: "A23 falsch".to_string(), id: 23, given_answers: vec![] },
                          Answer { text: "A24 falsch".to_string(), id: 24, given_answers: vec![] }],
            correct: 22,
            index: 1,
            total_questions: 3,
        },
        Question {
            text: "Frage 3".to_string(),
            answers: vec![Answer { text: "A31 falsch".to_string(), id: 31, given_answers: vec![] },
                          Answer { text: "A32 falsch".to_string(), id: 32, given_answers: vec![] },
                          Answer { text: "A33 richtig".to_string(), id: 33, given_answers: vec![] },
                          Answer { text: "A34 falsch".to_string(), id: 34, given_answers: vec![] }],
            correct: 33,
            index: 2,
            total_questions: 3,
        }]
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Answer not allowed: {0}")]
    AnswerNotAllowed(&'static str),

    #[error("Invalid game state {0}")]
    InvalidState(AppStatus),
}