import React from "react";
import './TimeBar.scss';

type TimeBarProps = {
  total_time: number,
  elapsed: number
}

type TimeBarState = {
  elapsed: number
}

const TIMER_PERIOD: number = 1000;

export class TimeBar extends React.Component<TimeBarProps, TimeBarState> {
  // const style = {
  //   "duration": total_time,
  //   "left": total_time - elapsed
  // }
  private interval: any;

  constructor(props: TimeBarProps) {
    super(props);
    this.state = {
      elapsed: props.elapsed
    }
    this.interval = null;
  }

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
      animationDelay: -this.state.elapsed + "ms"
    };
    return (
      <div className="timebar-div">
        <div key={Math.random()} className="timebar" style={style}>
          {this.state.elapsed / 1000}/{this.props.total_time / 1000} s
        </div>
      </div>
    );
  }
}