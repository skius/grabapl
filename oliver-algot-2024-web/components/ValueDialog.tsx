import React, {useEffect, useState} from 'react';
import Draggable from 'react-draggable';

export default function ValueDialog({
  onClose,
}: {
  onClose: (value: string | null) => void; // Changed the return type to void since it seems to be a callback
}) {
  const [userInput, setUserInput] = useState('');

  const handleSubmit = () => {
    onClose(userInput);
  };

  const handleCancel = () => {
    onClose(null);
  };

  useEffect(() => {
    const listener = (event: {code: string; preventDefault: () => void}) => {
      if (event.code === 'Enter' || event.code === 'NumpadEnter') {
        event.preventDefault();
        handleSubmit();
      }
    };
    document.addEventListener('keydown', listener);
    return () => {
      document.removeEventListener('keydown', listener);
    };
  }, [userInput]);

  return (
    <>
      <style>
        {`
          .dialogBackdrop {
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background-color: rgba(0, 0, 0, 0.2);
            display: flex;
            justify-content: center;
            align-items: center;
            z-index: 300;
          }

          .dialogContent {
            position: relative;
            background: white;
            padding: 30px 40px 15px 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
          }

          .close-icon {
            position: absolute;
            top: 10px;
            right: 10px;
            cursor: pointer;
            font-size: 1.5rem;
          }

          .center-group {
            display: flex;
            justify-content: center;
          }

          .modal-button {
            margin: 8px;
            background: #eff6ff;
            color: #1e40af;
            padding: 8px 16px;
            font-size: 0.875rem;
            border-radius: 8px;
            border: none;
            cursor: pointer;
          }

          .modal-button:hover {
            background-color: #bfdbfe;
          }

          .instructions {
            font-size: 0.875rem;
            margin-bottom: 5px;
          }

        `}
      </style>
      <div className="dialogBackdrop">
        <Draggable>
          <div className="dialogContent">
            <div className="close-icon" onClick={handleCancel}>
              <span className="material-icons-outlined">close</span>
            </div>
            <div className="center-group">
              <div className="instructions">Set the node value</div>
            </div>
            <input
              type="text"
              value={userInput}
              onChange={e => setUserInput(e.target.value)}
              autoFocus
              style={{padding: '10px', margin: '10px 0', width: '100%'}}
            />
            <div className="center-group">
              <button className="modal-button" onClick={handleSubmit}>
                Submit
              </button>
              <button className="modal-button" onClick={handleCancel}>
                Cancel
              </button>
            </div>
          </div>
        </Draggable>
      </div>
    </>
  );
}
