import React from 'react';
import './GameButton.scss';

type ButtonProps = {
  onClick: (event: any) => void,
  correct: boolean,
  wrong: boolean,
  selected: boolean,
  text: string
}

export const GameButton: React.FC<ButtonProps> = ({onClick, correct, wrong, selected, text}) => {
  return (
    <button
      className={`gamebutton ${correct && 'correct'} ${wrong && 'wrong'} ${selected && 'selected'}`}
      onClick={onClick}>
      {text}
    </button>
  );
}