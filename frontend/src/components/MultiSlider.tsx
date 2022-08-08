import React from "react";

export type MultiSliderElement = {
  name: string,
  description: string,
  value: number,
  min: number,
  max: number
}

type MultiSliderProps = {
  options: MultiSliderElement[],
  name: string,
  display: string,
  onChange: (name: string, value: number) => void,
}

export const MultiSlider: React.FC<MultiSliderProps> =
  ({options, name, display, onChange}) => {
    return (
      <div className="slider-container">
        <h3>{display}</h3>
        <div>
          {options.map((sm) => {
            return (
              <label key={sm.name}>
                <input type="range" value={sm.value} name={sm.name} min={sm.min} max={sm.max}
                       onChange={(e) => onChange(sm.name, Number(e.target.value))}/> {sm.description}
              </label>
            );
          })}
        </div>
      </div>
    );
  }