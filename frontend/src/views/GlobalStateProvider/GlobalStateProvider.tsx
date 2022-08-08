import React from "react";

const LOCALSTORE_STATE = "globalstate";

const defaultGlobalState = {
  user: "",
};

export interface GlobalStateContextType {
  state: {
    user: string
  }
  updateState: (newState: object) => void
}

export const globalStateContext = React.createContext<GlobalStateContextType | null>(null);

export const GlobalStateProvider: React.FC<React.ReactNode> = ({children}) => {
  const loadedState = {...defaultGlobalState, ...JSON.parse(window.localStorage.getItem(LOCALSTORE_STATE) || "{}")}
  const [state, setState] = React.useState(loadedState);

  const updateState = (newState: object) => {
    const combined = {...state, ...newState};
    setState(combined);
    window.localStorage.setItem(LOCALSTORE_STATE, JSON.stringify(combined));
  }

  return (
    <globalStateContext.Provider value={{state, updateState}}>
        {children}
    </globalStateContext.Provider>
  )
}
