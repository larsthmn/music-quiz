import React from "react";
import './TimeBar.scss';

type TimeBarProps = {
  total_time: number,
  elapsed: number
  colorful: boolean
}

type TimeBarState = {
  elapsed: number
}

const TIMER_PERIOD: number = 1000;

export class TimeBar extends React.Component<TimeBarProps, TimeBarState> {
  private interval: any;

  constructor(props: TimeBarProps) {
    super(props);
    this.state = {
      elapsed: props.elapsed
    }
    this.interval = null;
  }

  // Counter is only for updating second counter, bar does not need regular updates
  decrementTimeRemaining = () => {
    if (this.state.elapsed < this.props.total_time) {
      this.setState({
        elapsed: Math.min(this.props.total_time, this.state.elapsed + TIMER_PERIOD)
      })
    } else {
      if (this.interval !== null) {
        clearInterval(this.interval);
        this.interval = null;
      }
    }
  }

  componentDidMount() {
    this.interval = setInterval(() => {
      this.decrementTimeRemaining();
    }, TIMER_PERIOD);
  }

  componentWillUnmount() {
    if (this.interval !== null) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }

  render() {
    const style = {
      animationDuration: this.props.total_time + "ms",
      animationDelay: -this.state.elapsed + "ms",
      animationName: this.props.colorful ? "timeanimation-color" : "timeanimation",
      backgroundColor: this.props.colorful ? "lightgrey" : "grey",
    };
    return (
      <div className="timebar-container">
        <div key={Math.random()} className="timebar" style={style}/>
        <div className="timebar-text">{Math.round((this.props.total_time - this.state.elapsed) / 1000)}s</div>
        {/*<div className="timebar-text">{this.props.total_time - this.state.elapsed}ms</div>*/}
      </div>
    );
  }
}