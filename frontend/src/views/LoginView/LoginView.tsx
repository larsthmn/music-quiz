import React, {useContext, useState} from "react";
import './LoginView.scss'
import {useNavigate} from "react-router-dom";
import {globalStateContext, GlobalStateContextType} from "../GlobalStateProvider/GlobalStateProvider";

export const LoginView: React.FC = () => {

  const {state, updateState} = useContext(globalStateContext) as GlobalStateContextType;
  const [name, setName] = useState<string | null>(state.user);

  let nav = useNavigate();

  return (
    <div>
      <h1>Namen eingeben</h1>
      <input className="name_input" onChange={(e) => setName(e.target.value)} defaultValue={name ? name : ""}/>
      <button className={"login_button"} type="submit" onClick={
        () => {
          if (name !== "" && name != null) {
            updateState({user: name});
            nav('/game');
          }
        }
      }>Los
      </button>
      <button className={"preferences_button"} type="submit" onClick={() => nav('/control')}>
        Steuerung
      </button>
    </div>
  )
};