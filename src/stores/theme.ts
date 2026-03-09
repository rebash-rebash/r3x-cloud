import { createSignal } from "solid-js";

type Theme = "dark" | "light";

const stored = localStorage.getItem("r3x-theme") as Theme | null;
const [theme, setThemeSignal] = createSignal<Theme>(stored || "dark");

function applyTheme(t: Theme) {
  document.documentElement.setAttribute("data-theme", t);
  localStorage.setItem("r3x-theme", t);
}

// Apply on load
applyTheme(theme());

export function toggleTheme() {
  const next = theme() === "dark" ? "light" : "dark";
  setThemeSignal(next);
  applyTheme(next);
}

export { theme };
