import "./app.css";
import ClaudeStatus from "./ClaudeStatus.svelte";
import { mount } from "svelte";

const app = mount(ClaudeStatus, {
  target: document.getElementById("app")!,
});

export default app;
