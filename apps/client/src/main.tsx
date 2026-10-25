import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import "./styles.css";
import { Workspace } from "./Workspace";

createRoot(document.getElementById("root")!).render(
  <StrictMode><Workspace native={isTauri()} /></StrictMode>,
);
