import React, {useState} from "react";
import './LoginView.scss'

export const LoginView: React.FC<{ onSubmit: (name: string) => void }> = ({onSubmit}) => {

  const [name, setName] = useState("");

  return (
    <div>
      <h1>Namen eingeben</h1>
      <input onChange={(e) => setName(e.target.value)}/>
      <button className={"login_button"} type="submit" onClick={
        () => {
          if (name !== "" && name !== "admin") {
            onSubmit(name);
          }
        }
      }>Los
      </button>
      <button className={"preferences_button"} type="submit" onClick={
        () => {
          onSubmit("admin");
        }
      }>Einstellungen
      </button>
    </div>
  )
};