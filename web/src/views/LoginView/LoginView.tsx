import React, {useState} from "react";
import './LoginView.scss'

export const LoginView: React.FC<{ onSubmit: (name: string, admin: boolean) => void, username: string | null }> = ({onSubmit, username}) => {

  const [name, setName] = useState<string | null>(username);

  return (
    <div>
      <h1>Namen eingeben</h1>
      <input className="name_input" onChange={(e) => setName(e.target.value)} defaultValue={name ? name : ""}/>
      <button className={"login_button"} type="submit" onClick={
        () => {
          if (name !== "" && name !== "admin" && name != null) {
            onSubmit(name, false);
          }
        }
      }>Los
      </button>
      <button className={"preferences_button"} type="submit" onClick={
        () => {
          onSubmit("admin", true);
        }
      }>Steuerung
      </button>
    </div>
  )
};