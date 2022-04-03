import React from 'react';
import './GameView.scss';
import {GameButton} from "../../components/GameButton";
import {TimeBar} from "../../components/TimeBar";

type GameProps = {
  username: string,
  exit: () => void,
  timediff: number // local time - backend time
}

const MIN_POLL_RATE = 200;
const MAX_POLL_RATE = 2000;

export class GameView extends React.Component<GameProps, any> {
  private timer: NodeJS.Timeout | null;
  constructor(props: GameProps) {
    super(props);
    this.state = {
      data: {
        "status": "Ready",
        "action_start": 0,
        "next_action": 10000,
        "index": 1,
        "current_question": {
          "text": "Hier steht die Frage",
          "answers": [
            {"text": "Antwort 1", "id": 1, "selected_by": ["user1"]},
            {"text": "Antwort 2", "id": 2, "selected_by": ["test"]},
            {"text": "Antwort 3", "id": 3, "selected_by": []},
            {"text": "Antwort 4", "id": 4, "selected_by": []}],
          "correct": -1
        }
      }
    };
    this.timer = null;
  }

  poll() {
    // stop running timers
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    // get new state and start timer for next automatic poll
    fetch("/get_state")
      .then((response) => response.json(), () => {
        console.log("error on parsing json");
        this.timer = setTimeout(() => this.poll(), MIN_POLL_RATE); // retry after 100ms
      })
      .then((data) => {
        this.setState({data: data});
        const timeout: number = Math.max(MIN_POLL_RATE,
          Math.min(data.next_action - Date.now() - this.props.timediff, MAX_POLL_RATE));
        console.log("Timeout " + timeout);
        this.timer = setTimeout(() => this.poll(), timeout);
      }, () => {
        console.log("error on getting state");
        this.timer = setTimeout(() => this.poll(), MIN_POLL_RATE); // retry atfer 100ms
      });
  }

  componentDidMount() {
    this.poll();
  }

  componentWillUnmount() {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  onClick(id: number) {
    const data = {
      "id": id,
      "timestamp": Date.now() - this.props.timediff,
      "user": this.props.username
    }
    fetch("/press_button", {
      'method': 'POST',
      'headers': {
        'Content-Type': 'application/json',
      },
      'body': JSON.stringify(data)
    }).then((response) => {
      // console.log(response);
    }, (error) => {
      console.log(error);
    });
    console.log("Pressed" + id);
    this.poll();
  }

  render() {
    const {data} = this.state;
    return (
      <div>
        <div>
          <h1>
            Hey {this.props.username}!
          </h1>
          <button
            className={'backbutton'}
            onClick={this.props.exit}>
          </button>
        </div>
        {(data.current_question !== null && (data.status === "InGameAnswerPending" || data.status === "InGameWaitForNextQuestion")) ?
          <><h2>
            {data.current_question.text}
            {data.status === "InGameAnswerPending" && " (Bitte antworten)"}
            {data.status === "InGameWaitForNextQuestion" && " (Zeit abgelaufen)"}
          </h2>
            <div className={'button_container'}>
              {data.current_question.answers.map((answer: { id: number; selected_by: string | any[] | null; text: string; }) => {
                return (
                  <GameButton onClick={() => {
                    this.onClick(answer.id);
                  }}
                              correct={answer.id === data.current_question.correct}
                              wrong={data.current_question.correct !== -1 && answer.id !== data.current_question.correct && answer.selected_by !== null ? answer.selected_by.includes(this.props.username) : false}
                              selected={answer.selected_by !== null ? answer.selected_by.includes(this.props.username) : false}
                              text={answer.text}>
                  </GameButton>
                );
              })}
              <TimeBar key={data.action_start} total_time={data.next_action - data.action_start} elapsed={Date.now() - data.action_start - this.props.timediff}/>
            </div>
          </>
          :
          <h2>Warte auf Spielstart...</h2>
        }
      </div>
    );
  }
}