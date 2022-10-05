import React from "react";

export type SingleSelectionElement = {
  name: string,
  description: string,
}

type SelectionProps = {
  options: SingleSelectionElement[],
  selected: string,
  name: string,
  display: string,
  onChange: (selected: string) => void
}

export const SingleSelection: React.FC<SelectionProps> =
  ({options, selected,  name, display, onChange}) => {
    return (
      <div className="radio-container">
        {options.map((sm) => {
          return (
            <label key={sm.name + selected}>
              <input defaultChecked={selected === sm.name} type="radio" value={sm.name} name={name}
                     onClick={() => onChange(sm.name)}/> {sm.description}
            </label>
          );
        })}
      </div>
    );
  }