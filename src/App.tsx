import { useCallback, useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type PermLabel = "loading" | "allowed" | "denied" | "unknown";

function App() {
  const [perm, setPerm] = useState<PermLabel>("loading");
  const [activity, setActivity] = useState("");

  const refreshPermission = useCallback(async () => {
    try {
      const allowed = await invoke<boolean>("get_ax_accessibility_allowed");
      setPerm(allowed ? "allowed" : "denied");
    } catch {
      setPerm("unknown");
    }
  }, []);

  useEffect(() => {
    void refreshPermission();
  }, [refreshPermission]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("ax-frontmost-changed", (e) => {
      setActivity(`Frontmost changed: ${e.payload}`);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function start() {
    try {
      await invoke("start_ax_frontmost_monitor");
      await refreshPermission();
      setActivity("Monitoring frontmost app… switch apps to see updates.");
    } catch (e) {
      await refreshPermission();
      setActivity(String(e));
    }
  }

  const permText =
    perm === "loading"
      ? "Checking…"
      : perm === "allowed"
        ? "Allowed"
        : perm === "denied"
          ? "Not allowed"
          : "Unknown (not macOS or error)";

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vite.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <p>
        <strong>Accessibility permission:</strong> {permText}
      </p>
      <p>Start listens for the frontmost macOS app (requires permission above).</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          void start();
        }}
      >
        <button type="submit">Start</button>
        <button type="button" onClick={() => void refreshPermission()}>
          Refresh status
        </button>
      </form>
      {activity ? <p>{activity}</p> : null}
    </main>
  );
}

export default App;
