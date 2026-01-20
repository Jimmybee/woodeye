import "./app.css";
import App from "./App.svelte";
import DebugWindow from "./lib/components/DebugWindow.svelte";
import { mount } from "svelte";

// Route to appropriate component based on URL path
const path = window.location.pathname;
const target = document.getElementById("app")!;

let app;
if (path === "/debug") {
  app = mount(DebugWindow, { target });
} else {
  app = mount(App, { target });
}

export default app;
