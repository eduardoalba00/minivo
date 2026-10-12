import "@fontsource-variable/geist";
import "@fontsource/instrument-serif/400.css";
import { createRoot } from "react-dom/client";
import { App } from "./App.jsx";
import "./style.css";

const root = document.getElementById("root");
createRoot(root).render(<App />);
