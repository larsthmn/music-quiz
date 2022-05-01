import React from "react";
import './ResultView.scss';

type ResultViewProps = {
  results: any[],
  small: boolean,
  title: string
}

export const ResultView: React.FC<ResultViewProps> = ({results, small, title}) => {
  let max = Math.max.apply(Math, results.map((r) => {
    return r.points;
  }));
  if (max <= 0) max = 1;
  return (
    <div className={small ? "small-resultview" : "big-resultview"}>
      <h2>{title}</h2>
      {results.map((res) => {
        return (
          <div className="result-bar-layout">
            <div className="player-name">{res.player}</div>
            <div className="result-bar-container">
              <div className="result-bar-background"/>
              <div className="result-bar" style={{width: String((res.points / max) * 100) + "%"}}/>
              <div className="result-text">{res.points} ({res.correct} / {res.answers_given})</div>
            </div>
          </div>
        );
      })}
    </div>);
}
