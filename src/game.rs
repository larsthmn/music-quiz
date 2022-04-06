use std::error::Error;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{fmt, thread};
use std::fmt::{Display, Formatter};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rocket::serde::{Deserialize, Serialize};
use crate::game::GameError::{AnswerNotAllowed, InvalidState};
use crate::Ready;

// todo: as param, configure by admin
const TIME_BETWEEN_ROUNDS: u64 = 30000;
const TIME_TO_ANSWER: u64 = 2000;
const TIME_BETWEEN_ANSWERS: u64 = 500;

#[derive(Serialize, Clone)]
pub struct Answer {
    pub text: String,
    pub id: u64,
    pub selected_by: Vec<String>,
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
            answers_given: 0
        }
    }
}

#[derive(PartialEq, Serialize, Copy, Clone, Debug, strum_macros::Display)]
pub enum AppStatus {
    Shutdown,
    Ready,
    InGameAnswerPending,
    InGameWaitForNextQuestion,
    BetweenRounds
}

// Main game management structure
#[derive(Serialize, Clone)]
pub struct GameState {
    pub status: AppStatus,
    pub action_start: u64,
    pub next_action: u64,
    pub current_question: Option<Question>,
    pub results: Vec<PlayerScore>
}

#[derive(Serialize, Deserialize)]
pub struct AnswerFromUser {
    id: u64,
    timestamp: u64,
    user: String
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            status: AppStatus::Shutdown,
            action_start: 0,
            next_action: 0,
            current_question: None,
            results: vec![]
        }
    }

    /// Receive an answer from a user
    pub fn give_answer(&mut self, answer: AnswerFromUser) -> Result<(), GameError> {
        if self.status != AppStatus::InGameAnswerPending {
            return Err(InvalidState(self.status))
        }
        // Check if answers already contain user somewhere
        if let Some(current_question) = &mut self.current_question {
            let user_has_selected = current_question.answers
                .iter()
                .any(|a| a.selected_by.iter().any(|u| *u == answer.user));

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
                ans.selected_by.push(answer.user.clone());
                println!("User {} selected {} at {}", answer.user, ans.text, answer.timestamp);
            } else {
                return Err(AnswerNotAllowed("Invalid ID"));
            }

        } else {
            return Err(AnswerNotAllowed("No current question"));
        }
        Ok(())
    }
}

fn get_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is < UNIX_EPOCH")
        .as_millis() as u64
}

fn game_round(state: &Arc<Mutex<GameState>>, questions: Vec<Question>) {
    let mut s = state.lock().unwrap();
    s.results = vec![];
    drop(s);

    // Init results of this round
    for question in questions {
        // Set new question
        let mut s = state.lock().unwrap();
        let mut q = question.clone();
        q.correct = 0;
        s.current_question = Some(q);
        let now = get_epoch_ms();
        s.action_start = now;
        s.next_action = now + TIME_TO_ANSWER;
        s.status = AppStatus::InGameAnswerPending;
        println!("Question no {} / {}", question.index + 1, question.total_questions);
        drop(s);

        // Wait for users to answer
        thread::sleep(Duration::from_millis(TIME_TO_ANSWER));

        // Evaluate answers
        let mut s = state.lock().unwrap();
        println!("Question no {} / {} finished!", question.index + 1, question.total_questions);
        s.status = AppStatus::InGameWaitForNextQuestion;
        if let Some(q)  = &mut s.current_question {
            // Publish correct index
            q.correct = question.correct;
        }
        if let Some(q)  = &s.current_question.clone() { // todo: does it work without clone somehow?
            for ans in &q.answers {
                for player in &ans.selected_by {
                    // find player in results
                    if !s.results.iter().any(|score| score.player == *player) {
                        s.results.push(PlayerScore::new(player.clone()));
                    }
                    let mut score = s.results
                        .iter_mut()
                        .find(|score| score.player == *player)
                        .expect("Player must be in Vector");
                    score.answers_given += 1;
                    if ans.id == q.correct {
                        score.correct += 1;
                        score.points += 100;
                    }
                }
            }
        }
        s.results.sort_by(|a, b| b.points.cmp(&a.points));
        let now = get_epoch_ms();
        s.action_start = now; // todo: maybe better to not use current timestamp but calculate from last
        s.next_action = now + TIME_BETWEEN_ANSWERS;
        drop(s);

        // Wait for next question
        thread::sleep(Duration::from_millis(TIME_BETWEEN_ANSWERS));
    }

    // show results
    let mut s = state.lock().unwrap();
    s.current_question = None;
    let now = get_epoch_ms();
    s.action_start = now;
    s.next_action = now + TIME_BETWEEN_ROUNDS;
    s.status = AppStatus::BetweenRounds;
    drop(s);

}

pub fn run(state: Arc<Mutex<GameState>>) {

    // Wait for start by admin?
    let mut s = state.lock().unwrap();
    s.status = Ready;
    s.results = vec![];
    let now = get_epoch_ms();
    s.action_start = now;
    s.next_action = now + TIME_BETWEEN_ROUNDS;
    s.current_question = None;
    drop(s);
    println!("Wait for start by admin");
    thread::sleep(Duration::from_secs(3));

    loop {
        // game start
        println!("Start round");
        let questions = vec![
            Question {
                text: "Frage 1".to_string(),
                answers: vec![Answer { text: "A11 richtig".to_string(), id: 11, selected_by: vec![] },
                              Answer { text: "A12 falsch".to_string(), id: 12, selected_by: vec![] },
                              Answer { text: "A13 falsch".to_string(), id: 13, selected_by: vec![] },
                              Answer { text: "A14 falsch".to_string(), id: 14, selected_by: vec![] }],
                correct: 11,
                index: 0,
                total_questions: 3
            },
            Question {
                text: "Frage 2".to_string(),
                answers: vec![Answer { text: "A21 falsch".to_string(), id: 21, selected_by: vec![] },
                              Answer { text: "A22 richtig".to_string(), id: 22, selected_by: vec![] },
                              Answer { text: "A23 falsch".to_string(), id: 23, selected_by: vec![] },
                              Answer { text: "A24 falsch".to_string(), id: 24, selected_by: vec![] }],
                correct: 22,
                index: 1,
                total_questions: 3
            },
            Question {
                text: "Frage 3".to_string(),
                answers: vec![Answer { text: "A31 falsch".to_string(), id: 31, selected_by: vec![] },
                              Answer { text: "A32 falsch".to_string(), id: 32, selected_by: vec![] },
                              Answer { text: "A33 richtig".to_string(), id: 33, selected_by: vec![] },
                              Answer { text: "A34 falsch".to_string(), id: 34, selected_by: vec![] }],
                correct: 33,
                index: 2,
                total_questions: 3
            }];
        game_round(&state, questions);
        println!("Round ended");

        thread::sleep(Duration::from_secs(10));

        // let mut s = state.lock().unwrap();
        // next_question(s.deref_mut());
        // drop(s);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Answer not allowed: {0}")]
    AnswerNotAllowed(&'static str),

    #[error("Invalid game state {0}")]
    InvalidState(AppStatus),
}