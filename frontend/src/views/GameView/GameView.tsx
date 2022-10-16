import React from 'react';
import './GameView.scss';
import {GameButton} from "../../components/GameButton";
import {TimeBar} from "../../components/TimeBar";
import {ResultView} from "../ResultView/ResultView";
import {Link} from 'react-router-dom';
import {globalStateContext} from "../GlobalStateProvider/GlobalStateProvider";
import {GameState} from "../../../../shared/GameState";
import {UserAnswerExposed} from "../../../../shared/UserAnswerExposed";
import {WebSocketMessage} from "../../../../shared/WebSocketMessage";
import {TimeAnswer} from "../../../../shared/TimeAnswer";
import {TimeRequest} from "../../../../shared/TimeRequest";
import {DEFAULT_GAME_STATE, SOCKET_CHECK_RATE, TIME_SYNC_PERIOD} from "./GameViewConstants";
import {config} from "../../constants";

enum SocketState {
  Connecting = 0, // Socket has been created. The connection is not yet open.
  Open = 1, // The connection is open and ready to communicate.
  Closing = 2, // The connection is in the process of closing.
  Closed = 3 //	The connection is closed or couldn't be opened.
}

const socketStateText : Record<SocketState, string> =  {
  [SocketState.Connecting]: "Verbinde...",
  [SocketState.Open]: "Verbunden",
  [SocketState.Closing]: "Wird geschlossen",
  [SocketState.Closed]: "Keine Verbindung"
};

type GameViewState = {
  gamestate: GameState,
  socket_state: SocketState,
  ping: number
}

export class GameView extends React.Component<any, GameViewState> {
  private mounted: boolean;
  private interval_time: ReturnType<typeof setInterval> | null;
  private interval_socket: ReturnType<typeof setInterval> | null;
  private timediff : number;
  private socket: WebSocket | undefined;

  static contextType = globalStateContext;

  constructor(props: any) {
    super(props);
    this.state = {
      gamestate: DEFAULT_GAME_STATE,
      ping: 0,
      socket_state: SocketState.Closed
    };
    this.connect = this.connect.bind(this);
    this.mounted = false;
    this.interval_time = null;
    this.interval_socket = null;
    this.timediff = 0;
  }

  componentDidMount() {
    // Interval to check time offset and ping
    this.interval_time = setInterval(() => {
      const now = Date.now();
      if (this.socket && this.socket.readyState === 1) {
        const time_request : TimeRequest = {now: now};
        const message : WebSocketMessage = {
          message_type: "Time",
          data: JSON.stringify(time_request)
        }
        this.socket.send(JSON.stringify(message));
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
    if (this.socket === undefined || (this.socket && this.socket.readyState === SocketState.Closed)) {
      this.socket = new WebSocket(config.WS_URL);

      this.socket.onmessage = (msg) => {
        const ws_msg : WebSocketMessage = JSON.parse(msg.data);
        switch (ws_msg.message_type) {
          case "GameState":
            this.setState({gamestate: JSON.parse(ws_msg.data)});
            break;

          case "Time":
            const time_ans : TimeAnswer = JSON.parse(ws_msg.data);
            this.timediff = time_ans.diff_receive;
            this.setState({ping: Date.now() - Number(time_ans.ts_received)})
            break;

          default:
            break;
        }
      }
    }
    this.setState({socket_state: this.socket ? this.socket.readyState : SocketState.Closed })
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
    const data = this.state.gamestate;
    const {state: context} = this.context;
    const socket_state: SocketState = this.state.socket_state;
    let content = <h2>Unbekannter Spielstatus...</h2>;

    if (data != null) {
      switch (data.status) {
        case "InGameAnswerPending":
        case "InGameWaitForNextQuestion":
          const buttons = data.current_question?.answers.map((answer: { id: string; text: string; }) => {
            const is_selected: boolean = data.given_answers?.find((x: UserAnswerExposed) => x.user === context.user && answer.id === x.answer_id) !== undefined;
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
                          markings={data.given_answers?.filter((a) => a.answer_id === answer.id && (!data.hide_answers || a.user === context.user)).map((a: { user: string; }) => String(a.user))}>
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

        case "Preparing":
          content = <h2>Runde wird vorbereitet...</h2>;
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
          <label className={`indicator ${SocketState[socket_state]}`}>{socket_state === SocketState.Open ? this.state.ping + " ms" : socketStateText[socket_state]}</label>
          <Link to='/'>
            <button className={'backbutton'}/>
          </Link>
        </div>
        {content}
      </div>
    );
  }
}
