import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
