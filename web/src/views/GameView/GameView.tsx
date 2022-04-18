import React from 'react';
import './GameView.scss';
import {GameButton} from "../../components/GameButton";
import {TimeBar} from "../../components/TimeBar";
import {ResultView} from "../ResultView/ResultView";
import { Link } from 'react-router-dom';
import {globalStateContext} from "../GlobalStateProvider/GlobalStateProvider";

const TIME_SYNC_PERIOD = 20000;
const MIN_POLL_RATE = 150;
const MAX_POLL_RATE = 10000;

export class GameView extends React.Component<any, any> {
  private timer: ReturnType<typeof setTimeout> | null;
  private mounted: boolean;
  private interval: ReturnType<typeof setInterval> | null;
  private timediff;

  static contextType = globalStateContext;

  constructor(props: any) {
    super(props);
    this.state = {data: {status: "Shutdown"}};
    this.mounted = false;
    this.timer = null;
    this.interval = null;
    this.timediff = 0;
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
          Math.min(data.next_action - Date.now() + this.timediff, MAX_POLL_RATE));
        console.log("parsed data, timeout = " + timeout);
        this.timer = setTimeout(() => this.poll(), timeout);
      }, () => {
        console.log("error on getting state");
        this.timer = setTimeout(() => this.poll(), MIN_POLL_RATE); // retry after 100ms
      });
  }

  componentDidMount() {
    this.interval = setInterval(() => {
      const now = Date.now();
      fetch("/get_time?now=" + now)
        .then((response) => response.json(), () => {
          console.log("error on parsing json");
        })
        .then((data) => {
          console.log("timediff " + data.diff_receive + "ms");
          // todo: better time synch, use roundtrip time or something
          this.timediff = data.diff_receive;
        }, () => {
          console.log("error on getting time");
        });
    }, TIME_SYNC_PERIOD);
    this.mounted = true;
    this.poll();
  }

  componentWillUnmount() {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    if (this.interval) {
      clearInterval(this.interval);
      this.interval = null;
    }
    this.mounted = false;
  }

  onClick(id: any) {
    const {state} = this.context;
    const data = {
      "id": id,
      "timestamp": Date.now() - this.timediff,
      "user": state.user
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
    const {state} = this.context;
    let content = <h2>Unbekannter Spielstatus...</h2>;

    if (data != null) {
      switch (data.status) {
        case "InGameAnswerPending":
        case "InGameWaitForNextQuestion":
          const buttons = data.current_question.answers.map((answer: { id: any; given_answers: any[] | null; text: string; }) => {
            const is_selected: boolean = data.given_answers?.find((x: any) => x.user === state.user && answer.id === x.answer_id);
            const is_correct_answer: boolean = answer.id === data.current_question.correct;
            const is_correct_known: boolean = data.current_question.correct !== 0;
            return (
              <GameButton key={answer.id} onClick={() => {
                this.onClick(answer.id);
              }}
                          correct={is_correct_known && is_correct_answer}
                          wrong={is_correct_known && !is_correct_answer && is_selected}
                          selected={is_selected}
                          text={answer.text}
                          markings={data.given_answers?.filter((a: any) => a.answer_id === answer.id).map((a: { user: string; }) => String(a.user))}>
              </GameButton>
            );
          });

          content =
            <div>
              <h3>{data.current_question?.index} / {data.current_question?.total_questions}</h3>
              <h2>
                {data.current_question.text}
                {data.status === "InGameAnswerPending" && " (Bitte antworten)"}
                {data.status === "InGameWaitForNextQuestion" && " (Zeit abgelaufen)"}
              </h2>
              <div className={'button_container'}>
                <TimeBar key={Math.random()} total_time={data.next_action - data.action_start}
                         elapsed={Date.now() - data.action_start - this.timediff}
                         colorful={data.status === "InGameAnswerPending"}/>
                {buttons}
              </div>
            </div>
          break;

        case "BetweenRounds":
          content = <ResultView results={data.players}/>;
          break;

        case "Ready":
          content = <h2>Warte auf Spielstart...</h2>;
          break;

        case "BeforeGame":
          content =
            <div>
              <h2>Bereitmachen</h2>
              <TimeBar key={Math.random()} total_time={data.next_action - data.action_start}
                       elapsed={Date.now() - data.action_start - this.timediff}
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
            Hey {state.user}!
          </h1>
          <Link to='/'>
            <button className={'backbutton'} />
          </Link>
        </div>
        {content}
      </div>
    );
  }
}
