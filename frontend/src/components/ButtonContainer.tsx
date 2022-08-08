import React from "react";
// import {GameButton} from "./GameButton";
//
// type ButtonContainerProps = {
//   data: any
// }
//
// export const ButtonContainer: React.FC<ButtonContainerProps> = (data) => {
//   return (
//     <div>
//       <h2>
//         {data.current_question !== null ? data.current_question.text : "Warte auf Frage..."}
//         {data.status == "InGameAnswerPending" && " (Bitte antworten)"}
//         {data.status == "InGameWaitForNextQuestion" && " (Zeit abgelaufen)"}
//       </h2>
//       <div className={'button_container'}>
//         {data.current_question.answers.map((answer: { id: number; selected_by: string | any[] | null; text: string; }) => {
//           return (
//             <GameButton onClick={() => {
//               this.onClick(answer.id)
//             }}
//                         correct={answer.id === data.current_question.correct}
//                         wrong={data.correct !== -1 && answer.selected_by !== null ? answer.selected_by.includes(this.props.username) : false}
//                         selected={answer.selected_by !== null ? answer.selected_by.includes(this.props.username) : false}
//                         text={answer.text}>
//             </GameButton>
//           );
//         })}
//       </div>
//     </div>
//   );
// }