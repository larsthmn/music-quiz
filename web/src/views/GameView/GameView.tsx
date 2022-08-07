import React from 'react';
import './GameView.scss';
import {GameButton} from "../../components/GameButton";
import {TimeBar} from "../../components/TimeBar";
import {ResultView} from "../ResultView/ResultView";
import {Link} from 'react-router-dom';
import {globalStateContext} from "../GlobalStateProvider/GlobalStateProvider";
import {GameState} from "../../../../bindings/GameState";
import {UserAnswerExposed} from "../../../../bindings/UserAnswerExposed";

const TIME_SYNC_PERIOD = 1000;
const MIN_POLL_RATE = 150;
const MAX_POLL_RATE = 1000;

export class GameView extends React.Component<any, GameState> {
  private timer: ReturnType<typeof setTimeout> | null;
  private mounted: boolean;
  private interval: ReturnType<typeof setInterval> | null;
  private timediff;

  static contextType = globalStateContext;

  constructor(props: any) {
    super(props);
    this.state = {
      status: "Shutdown",
      action_start: BigInt(0),
      next_action: BigInt(0),
      current_question: null,
      given_answers: [],
      players: [],
      hide_answers: false
    };
    // this.state = {
    //   status: "InGameAnswerPending",
    //   action_start: BigInt(1659900273643),
    //   next_action: BigInt(1659900278643),
    //   current_question: {
    //     text: "Wie heißt der Titel?",
    //     answers: [
    //       {
    //         text: "The Bottom",
    //         id: "The Bottom"
    //       },
    //       {
    //         text: "Blind Man",
    //         id: "Blind Man"
    //       },
    //       {
    //         text: "Help",
    //         id: "Help"
    //       },
    //       {
    //         text: "MC Thunder",
    //         id: "MC Thunder"
    //       }
    //     ],
    //     correct: null,
    //     solution: null,
    //     index: 1,
    //     total_questions: 5
    //   },
    //   players: [
    //     {
    //       player: "Lars",
    //       points: 100,
    //       correct: 1,
    //       answers_given: 1,
    //       last_points: 10,
    //       // last_points: 60,
    //       last_time: 0.3,
    //     }
    //   ],
    //   given_answers: [
    //     {
    //       answer_id: "Help",
    //       user: "Lars",
    //       ts: BigInt(1659900277055)
    //     }
    //   ],
    //   hide_answers: false
    // }

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
        this.setState(data);
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

  onClick(id: string) {
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
    const data = this.state;
    const {state} = this.context;
    let content = <h2>Unbekannter Spielstatus...</h2>;

    if (data != null) {
      switch (data.status) {
        case "InGameAnswerPending":
        case "InGameWaitForNextQuestion":
          const buttons = data.current_question?.answers.map((answer: { id: string; text: string; }) => {
            const is_selected: boolean = data.given_answers?.find((x: UserAnswerExposed) => x.user === state.user && answer.id === x.answer_id) != undefined;
            const is_correct_answer: boolean = answer.id === data.current_question?.correct;
            const is_correct_known: boolean = data.current_question?.correct !== null;
            return (
              <GameButton key={answer.id} onClick={() => {
                this.onClick(answer.id);
              }}
                          correct={is_correct_known && is_correct_answer}
                          wrong={is_correct_known && !is_correct_answer && is_selected}
                          selected={is_selected}
                          text={answer.text}
                          markings={data.given_answers?.filter((a) => a.answer_id === answer.id && (!data.hide_answers || a.user == state.user)).map((a: { user: string; }) => String(a.user))}>
              </GameButton>
            );
          });

          content =
            <div>
              <h2>
                [{data.current_question !== null ? (data.current_question.index + 1) : ""} / {data.current_question?.total_questions}]&nbsp;
                {data.status === "InGameAnswerPending" && data.current_question?.text}
                {data.status === "InGameWaitForNextQuestion" && "Lösung: " + data.current_question?.solution}
              </h2>
              <div className={'button_container'}>
                <TimeBar key={Math.random()} total_time={Number(data.next_action - data.action_start)}
                         elapsed={Date.now() - Number(data.action_start) - this.timediff}
                         colorful={data.status === "InGameAnswerPending"}/>
                {buttons}
              </div>
              <ResultView title="Punktestand" small={true} results={data.players}/>
            </div>
          break;

        case "BetweenRounds":
          content = <ResultView title="Endstand" small={false} results={data.players}/>;
          break;

        case "Ready":
          content = <h2>Warte auf Spielstart...</h2>;
          break;

        case "BeforeGame":
          content =
            <div>
              <h2>Bereitmachen</h2>
              <TimeBar key={Math.random()} total_time={Number(data.next_action - data.action_start)}
                       elapsed={Date.now() - Number(data.action_start) - this.timediff}
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
          <Link to='/'>
            <button className={'backbutton'}/>
          </Link>
        </div>
        {content}
      </div>
    );
  }
}
