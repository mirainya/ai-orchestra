import { Sun, Moon } from "lucide-react";
import { useThemeStore } from "../stores/theme";

export function ThemeToggle() {
  const { dark, toggle } = useThemeStore();
  return (
    <button
      onClick={toggle}
      className="p-2 rounded-lg transition-colors"
      style={{
        background: "var(--bg-tertiary)",
        color: "var(--text-primary)",
      }}
      title={dark ? "切换到浅色模式" : "切换到深色模式"}
    >
      {dark ? <Sun size={16} /> : <Moon size={16} />}
    </button>
  );
}
