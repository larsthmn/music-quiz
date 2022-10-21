import React, {useContext, useState} from "react";
import './LoginView.scss'
import {useNavigate} from "react-router-dom";
import {globalStateContext, GlobalStateContextType} from "../GlobalStateProvider/GlobalStateProvider";

export const LoginView: React.FC = () => {

  const {state, updateState} = useContext(globalStateContext) as GlobalStateContextType;
  const [name, setName] = useState<string | null>(state.user);

  let nav = useNavigate();

  return (
    <div className={"login-view"}>
      {/*<h1>Namen eingeben</h1>*/}
      <input className="form__field" onChange={(e) => setName(e.target.value)} defaultValue={name ? name : ""} placeholder={"Name"} id={'name'} name={"name"}/>
      <label htmlFor="name" className="form__label">Name</label>
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