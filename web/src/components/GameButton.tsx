import React from 'react';
import './GameButton.scss';

type ButtonProps = {
  onClick: (event: any) => void,
  correct: boolean,
  wrong: boolean,
  selected: boolean,
  text: string,
  markings: undefined | string[]
}

export const GameButton: React.FC<ButtonProps> = ({onClick, correct, wrong, selected, text, markings}) => {
  return (
    <button
      className={`button-element ${correct && 'correct'} ${wrong && 'wrong'} ${selected && 'selected'}`}
      onClick={onClick}>
      <div className="button-text">
      {text}
      </div>
      <div className="button-mark-container">
        {markings?.map((user: string) => {return (
          <div className="button-mark">{user} </div>
        );})}
      </div>
    </button>
  );
}