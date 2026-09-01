import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import { resolveWindowTarget } from "./windowTarget";

// Bug-fix 2026-05-18: launcher was crashing intermittently with no console
// trail. Capture both synchronous renderer errors and floating-promise
// rejections so the next crash leaves evidence — and so a single rejection
// doesn't tear down the entire WebView via the browser's default handling.
if (typeof window !== "undefined") {
  window.addEventListener("error", (event) => {
    console.error("[keynova:window-error]", event.message, event.error);
  });
  window.addEventListener("unhandledrejection", (event) => {
    console.error("[keynova:unhandled-rejection]", event.reason);
  });
}

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

// The two windows are code-split here rather than at the Vite entry: one
// `index.html`, one build, and the settings webview never parses the launcher's
// palette / xterm / markdown chunks because it never imports them. `index.css`
// stays static — both windows are styled by the same variable set.
const tree =
  resolveWindowTarget(window.location.search) === "settings"
    ? import("./windows/SettingsWindow").then((m) => <m.SettingsWindow />)
    : import("./App").then((m) => <m.default />);

void tree.then((element) => {
  root.render(<React.StrictMode>{element}</React.StrictMode>);
});
