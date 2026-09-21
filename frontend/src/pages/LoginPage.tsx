import { FormEvent, useState } from "react";

const VALID_USERNAME = "qquway";
const VALID_PASSWORD = "@Porta123";
const SESSION_KEY = "wa_authed";

export function isAuthenticated(): boolean {
  return sessionStorage.getItem(SESSION_KEY) === "1";
}

export function logout(): void {
  sessionStorage.removeItem(SESSION_KEY);
  window.location.reload();
}

export default function LoginPage({ onSuccess }: { onSuccess: () => void }) {
  const [selected, setSelected] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [shake, setShake] = useState(false);

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (username === VALID_USERNAME && password === VALID_PASSWORD) {
      sessionStorage.setItem(SESSION_KEY, "1");
      onSuccess();
      return;
    }
    setError("The username or password is incorrect. Try again.");
    setShake(true);
    setTimeout(() => setShake(false), 400);
  }

  return (
    <div className="xp-wallpaper xp-welcome">
      <div className="xp-welcome-top" />
      <div className="xp-welcome-main">
        <div className="xp-welcome-left">
          <div className="xp-welcome-logo">Work Assistant</div>
          <div className="xp-welcome-tagline">To begin, click your user name</div>
        </div>
        <div className="xp-welcome-divider" />
        <div className="xp-welcome-right">
          {!selected ? (
            <button
              type="button"
              className="xp-user-tile"
              onClick={() => setSelected(true)}
              autoFocus
            >
              <span className="xp-user-icon">👤</span>
              <span className="xp-user-name">qquway</span>
            </button>
          ) : (
            <form className={`xp-user-tile xp-user-tile-active ${shake ? "xp-shake" : ""}`} onSubmit={handleSubmit}>
              <span className="xp-user-icon">👤</span>
              <div className="xp-user-fields">
                <span className="xp-user-name">qquway</span>
                <input
                  type="text"
                  className="xp-hidden-username"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  autoComplete="username"
                  tabIndex={-1}
                  aria-hidden="true"
                />
                <div className="xp-password-row">
                  <input
                    type="password"
                    placeholder="Password"
                    value={password}
                    onChange={(e) => {
                      setPassword(e.target.value);
                      setUsername(VALID_USERNAME);
                    }}
                    autoFocus
                    autoComplete="current-password"
                  />
                  <button type="submit" className="xp-arrow-btn" aria-label="Log in">
                    ▶
                  </button>
                </div>
                {error && <div className="xp-login-error">{error}</div>}
              </div>
            </form>
          )}
        </div>
      </div>
      <div className="xp-welcome-bottom">
        <span>After logging on, you can add reminders and notes about your day.</span>
      </div>
    </div>
  );
}
