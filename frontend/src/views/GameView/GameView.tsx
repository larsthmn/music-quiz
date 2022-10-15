import React from 'react';
import './GameView.scss';
import {GameButton} from "../../components/GameButton";
import {TimeBar} from "../../components/TimeBar";
import {ResultView} from "../ResultView/ResultView";
import {Link} from 'react-router-dom';
import {globalStateContext} from "../GlobalStateProvider/GlobalStateProvider";
import {GameState} from "../../../../shared/GameState";
import {UserAnswerExposed} from "../../../../shared/UserAnswerExposed";
import {DataType} from "../../../../shared/DataType";
import {WebSocketMessage} from "../../../../shared/WebSocketMessage";
import {TimeAnswer} from "../../../../shared/TimeAnswer";
import {TimeRequest} from "../../../../shared/TimeRequest";

const TIME_SYNC_PERIOD = 1000;
const SOCKET_CHECK_RATE = 1000;

(BigInt.prototype as any).toJSON = function () {
  return Number(this);
};

enum SocketState {
  Connecting = 0, // Socket has been created. The connection is not yet open.
  Open = 1, // The connection is open and ready to communicate.
  Closing = 2, // The connection is in the process of closing.
  Closed = 3 //	The connection is closed or couldn't be opened.
}

export class GameView extends React.Component<any, GameState> {
  private mounted: boolean;
  private interval_time: ReturnType<typeof setInterval> | null;
  private interval_socket: ReturnType<typeof setInterval> | null;
  private timediff : bigint;
  private socket: WebSocket | undefined;

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
    this.connect = this.connect.bind(this);
    this.mounted = false;
    this.interval_time = null;
    this.interval_socket = null;
    this.timediff = BigInt(0);
  }

  componentDidMount() {
    this.interval_time = setInterval(() => {
      const now = Date.now();
      if (this.socket && this.socket.readyState === 1) {
        const time_request : TimeRequest = {now: BigInt(now)};
        const message : WebSocketMessage = {
          message_type: "Time",
          data: JSON.stringify(time_request)
        }
        this.socket.send(JSON.stringify(message));
        console.log("time requested");
      }
    }, TIME_SYNC_PERIOD);
    this.interval_socket = setInterval(this.connect, SOCKET_CHECK_RATE);
    this.mounted = true;
  }

  componentWillUnmount() {
    if (this.interval_time) {
      clearInterval(this.interval_time);
      this.interval_time = null;
    }
    if (this.interval_socket) {
      clearInterval(this.interval_socket);
      this.interval_socket = null;
    }
    this.mounted = false;
    this.socket?.close();
  }

  connect() {
    // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
    if (this.socket === undefined || (this.socket && this.socket.readyState === 3)) {
      this.socket = new WebSocket("ws://localhost:8000/ws");

      this.socket.onmessage = (msg) => {
        const ws_msg : WebSocketMessage = JSON.parse(msg.data);
        switch (ws_msg.message_type) {
          case "GameState":
            this.setState(JSON.parse(ws_msg.data));
            break;
          case "Time":
            const time_ans : TimeAnswer = JSON.parse(ws_msg.data);
            this.timediff = time_ans.diff_receive;
            console.log("timediff is " + this.timediff);
            break;
          default:
            break;
        }
      }
    }
  }

  onClick(id: string) {
    const {state} = this.context;
    const data = {
      "id": id,
      "timestamp": Date.now() - Number(this.timediff),
      "user": state.user
    }
    const message : WebSocketMessage = {
      message_type: "Answer",
      data: JSON.stringify(data)
    }
    if(this.socket) this.socket.send(JSON.stringify(message));
    console.log("Pressed " + id);
  }

  render() {
    const data = this.state;
    const {state} = this.context;
    let content = <h2>Unbekannter Spielstatus...</h2>;
    const socket_state: SocketState = this.socket ? (this.socket.readyState) : SocketState.Closed;

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
                ({data.current_question !== null ? (data.current_question.index + 1) : ""}/{data.current_question?.total_questions})&nbsp;
                {data.status === "InGameAnswerPending" && data.current_question?.text}
                {data.status === "InGameWaitForNextQuestion" && "Lösung: " + data.current_question?.solution}
              </h2>
              <div className={'button_container'}>
                <TimeBar key={Math.random()} total_time={Number(data.next_action - data.action_start)}
                         elapsed={Date.now() - Number(data.action_start - this.timediff)}
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
                       elapsed={Date.now() - Number(data.action_start - this.timediff)}
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
          <label className={`indicator ${SocketState[socket_state]}`}>{SocketState[socket_state]}</label>
          <Link to='/'>
            <button className={'backbutton'}/>
          </Link>
        </div>
        {content}
      </div>
    );
  }
}
