import {GameState} from "../../../../shared/GameState";

export const TIME_SYNC_PERIOD = 1000;
export const SOCKET_CHECK_RATE = 1000;

export const DEFAULT_GAME_STATE : GameState = {
  status: "Shutdown",
  action_start: 0,
  next_action: 0,
  current_question: null,
  given_answers: [],
  players: [],
  hide_answers: false
}

export const TEST_GAME_STATE : GameState = {
  status: "InGameAnswerPending",
  action_start: 1659900273643,
  next_action: 1659900278643,
  current_question: {
    text: "Wie heißt der Titel?",
    answers: [
      {
        text: "The Bottom",
        id: "The Bottom"
      },
      {
        text: "Blind Man",
        id: "Blind Man"
      },
      {
        text: "Help",
        id: "Help"
      },
      {
        text: "MC Thunder",
        id: "MC Thunder"
      }
    ],
    correct: "Blind Man",
    solution: null,
    index: 1,
    total_questions: 5
  },
  players: [
    {
      player: "Lars",
      points: 100,
      correct: 1,
      answers_given: 1,
      last_points: 10,
      // last_points: 60,
      last_time: 0.3,
    },
    {
      player: "Myje",
      points: 46,
      correct: 1,
      answers_given: 1,
      last_points: 10,
      // last_points: 60,
      last_time: 5.312,
    },
    {
      player: "Nils",
      points: 12,
      correct: 2,
      answers_given: 2,
      last_points: 100,
      // last_points: 60,
      last_time: 1.312,
    }
  ],
  given_answers: [
    {
      answer_id: "Help",
      user: "Lars",
      ts: 1659900277055
    }
  ],
  hide_answers: false
}