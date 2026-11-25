import { useState, type SubmitEvent } from "react";
import { errorText, login, register } from "../../lib/api";
import { icons } from "../../lib/icons";
import "./Auth.css";

type Props = {
  title: string;
  /** Sem usuário ainda não existe conta: mostra o cadastro. */
  username: string | null;
  onDone: () => void;
};

export default function Auth({ title, username, onDone }: Props) {
  const creating = username === null;
  const [name, setName] = useState(username ?? "");
  const [password, setPassword] = useState("");
  const [repeat, setRepeat] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    if (creating && password !== repeat) {
      setError("As senhas não são iguais");
      return;
    }
    setBusy(true);
    setError("");
    try {
      await (creating ? register : login)(name, password);
      onDone();
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  };

  return (
    <div className="auth">
      <form className="authCard" onSubmit={submit}>
        <span className="navLogo">{icons.server}</span>
        <h1>{title}</h1>
        <p>{creating ? "Crie a conta do servidor" : "Entre para continuar"}</p>

        <label className="field">
          <span>Usuário</span>
          <input
            required
            maxLength={40}
            autoFocus={creating}
            autoComplete="username"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <label className="field">
          <span>Senha{creating && " (mínimo de 6 caracteres)"}</span>
          <input
            type="password"
            required
            minLength={creating ? 6 : undefined}
            maxLength={200}
            autoFocus={!creating}
            autoComplete={creating ? "new-password" : "current-password"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        {creating && (
          <label className="field">
            <span>Repita a senha</span>
            <input
              type="password"
              required
              autoComplete="new-password"
              value={repeat}
              onChange={(e) => setRepeat(e.target.value)}
            />
          </label>
        )}

        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <button className="button primary" disabled={busy}>
          {creating ? "Criar conta" : "Entrar"}
        </button>
      </form>
    </div>
  );
}
