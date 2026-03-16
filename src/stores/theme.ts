import { create } from "zustand";

interface ThemeState {
  dark: boolean;
  toggle: () => void;
  init: () => void;
}

export const useThemeStore = create<ThemeState>((set, get) => ({
  dark: true,
  toggle: () => {
    const next = !get().dark;
    set({ dark: next });
    document.documentElement.classList.toggle("dark", next);
    localStorage.setItem("theme", next ? "dark" : "light");
  },
  init: () => {
    const saved = localStorage.getItem("theme");
    const dark = saved ? saved === "dark" : true;
    set({ dark });
    document.documentElement.classList.toggle("dark", dark);
  },
}));
