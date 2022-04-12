import React, {useEffect, useState} from 'react';
import './GameView.scss';
import {GameButton} from "../../components/GameButton";
import {TimeBar} from "../../components/TimeBar";
import {ResultView} from "../ResultView/ResultView";

type GameProps = {
  username: string,
  exit: () => void,
  timediff: number // local time - backend time
}

const MIN_POLL_RATE = 200;
const MAX_POLL_RATE = 2000;

export class GameView extends React.Component<GameProps, any> {
  private timer: NodeJS.Timeout | null;
  private mounted: boolean;

  constructor(props: GameProps) {
    super(props);
    this.state = {data: {status: "Shutdown"}};
    // this.state = {
    //   data: {
    //     "status": "BetweenRounds",
    //     "action_start": 1649273413432,
    //     "next_action": 1649273443432,
    //     "current_question": null,
    //     "results": [
    //       {
    //         "player": "Lars ",
    //         "points": 100,
    //         "correct": 0,
    //         "answers_given": 3
    //       },
    //       {
    //         "player": "Spacken",
    //         "points": 335,
    //         "correct": 1,
    //         "answers_given": 3
    //       }]
    //   }}
    this.mounted = false;
    this.timer = null;
  }

  poll() {
    if (this.mounted) {
      this.parseResponse(fetch("/get_state"));
    }
  };

  parseResponse(promise: Promise<Response>) {
    // stop running timers
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    promise.then((response) => response.json(), () => {
      console.log("error on parsing json");
      this.timer = setTimeout(() => this.poll(), MIN_POLL_RATE); // retry after 100ms
    })
      .then((data) => {
        this.setState({data: data});
        const timeout: number = Math.max(MIN_POLL_RATE,
          Math.min(data.next_action - Date.now() + this.props.timediff, MAX_POLL_RATE));
        console.log("parsed data, timeout = " + timeout);
        this.timer = setTimeout(() => this.poll(), timeout);
      }, () => {
        console.log("error on getting state");
        this.timer = setTimeout(() => this.poll(), MIN_POLL_RATE); // retry after 100ms
      });
  }

  componentDidMount() {
    this.mounted = true;
    this.poll();
  }

  componentWillUnmount() {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.mounted = false;
  }

  onClick(id: number) {
    const data = {
      "id": id,
      "timestamp": Date.now() - this.props.timediff,
      "user": this.props.username
    }
    this.parseResponse(fetch("/press_button", {
      'method': 'POST',
      'headers': {
        'Content-Type': 'application/json',
      },
      'body': JSON.stringify(data)
    }))
    console.log("Pressed" + id);
  }

  render() {
    const {data} = this.state;
    let content = <h2>Unbekannter Spielstatus...</h2>;

    if (data != null) {
      switch (data.status) {
        case "InGameAnswerPending":
        case "InGameWaitForNextQuestion":
          content =
            <div>
              <h2>
                {data.current_question.text}
                {data.status === "InGameAnswerPending" && " (Bitte antworten)"}
                {data.status === "InGameWaitForNextQuestion" && " (Zeit abgelaufen)"}
              </h2>
              <div className={'button_container'}>
                <TimeBar key={Math.random()} total_time={data.next_action - data.action_start}
                         elapsed={Date.now() - data.action_start - this.props.timediff}
                         colorful={data.status === "InGameAnswerPending"}/>
                {data.current_question.answers.map((answer: { id: number; given_answers: any[] | null; text: string; }) => {
                  return (
                    <GameButton key={answer.id} onClick={() => {
                      this.onClick(answer.id);
                    }}
                                correct={answer.id === data.current_question.correct}
                                wrong={data.current_question.correct !== 0
                                        && answer.id !== data.current_question.correct
                                        && answer.given_answers !== null ? answer.given_answers?.find(x => x.name === this.props.username) : false}
                                selected={answer.given_answers !== null ? answer.given_answers?.find(x => x.name === this.props.username) : false}
                                text={answer.text}
                                markings={answer.given_answers?.map(a => String(a.name))}>
                    </GameButton>
                  );
                })}
              </div>
            </div>
          break;

        case "BetweenRounds":
          content = <ResultView results={data.results}/>;
          break;

        case "Ready":
          content = <h2>Warte auf Spielstart...</h2>;
          break;

        case "BeforeGame":
          content =
            <div>
              <h2>Bereitmachen</h2>
              <TimeBar key={Math.random()} total_time={data.next_action - data.action_start}
                       elapsed={Date.now() - data.action_start - this.props.timediff}
                       colorful={true}/>
            </div>;
          break;

        case "Shutdown":
        default:
          content = <h2>Warte auf Server...</h2>;
          break;
      }
    }

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
        {content}
      </div>
    );
  }
}
