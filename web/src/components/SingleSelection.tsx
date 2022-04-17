import React from "react";

export type Selection = {
  name: string,
  description: string,
}

type SelectionProps = {
  options: Selection[],
  selected: string,
  name: string,
  display: string,
  onChange: (selected: string) => void
}

export const SingleSelection: React.FC<SelectionProps> =
  ({options, selected,  name, display, onChange}) => {
    return (
      <div className="radio-container">
        <h3>{display}</h3>
        {options.map((sm) => {
          return (
            <label key={sm.name}>
              <input defaultChecked={selected === sm.name} type="radio" value={sm.name} name={name}
                     onClick={() => onChange(sm.name)}/> {sm.description}
            </label>
          );
        })}
      </div>
    );
  }