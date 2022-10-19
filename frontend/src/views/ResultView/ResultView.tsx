import React from "react";
import './ResultView.scss';
import type { PlayerScoreAPI } from '../../../../shared/PlayerScoreAPI'

type ResultViewProps = {
  results: PlayerScoreAPI[],
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
      <h1>{title}</h1>
      {results.map((res, index) => {
        return (
          <div key={res.player} className="result-bar-layout">
            <label className={"player-pos pos-" + (index + 1)}>{index + 1}</label>
            <label className="player-name">{res.player}</label>
            <div className="result-bar-container">
              <div className="result-bar-background">
                <div className="result-bar" style={{width: String((res.points / max) * 100) + "%"}}/>
                <div className="result-bar alt-color" style={{width: String(((res.last_points || 0) / max) * 100) + "%"}}/>
                <div className="result-text">
                  <label className="result-text-left">{res.points} ({res.correct} / {res.answers_given})</label>
                  <label className="result-text-right">{res.last_points !== null ? <div className="result-text-right">{res.last_time?.toFixed(2)}s ({res.last_points === 0 ? "falsch" : "+" + res.last_points})</div> : null}</label>
                </div>
              </div>

            </div>
          </div>
        );
      })}
    </div>);
}
