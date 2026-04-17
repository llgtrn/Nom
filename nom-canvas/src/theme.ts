export interface Theme {
  name: string;
  variables: Record<string, string>;
}

// Default dark theme (from design-system/nomcanvas/MASTER.md)
export const DARK_THEME: Theme = {
  name: "dark",
  variables: {
    "--color-primary": "#1E293B",
    "--color-secondary": "#334155",
    "--color-cta": "#22C55E",
    "--color-background": "#0F172A",
    "--color-text": "#F8FAFC",
  },
};

// Light theme variant
export const LIGHT_THEME: Theme = {
  name: "light",
  variables: {
    "--color-primary": "#F1F5F9",
    "--color-secondary": "#E2E8F0",
    "--color-cta": "#16A34A",
    "--color-background": "#FFFFFF",
    "--color-text": "#0F172A",
  },
};

// High contrast theme
export const HIGH_CONTRAST_THEME: Theme = {
  name: "high-contrast",
  variables: {
    "--color-primary": "#000000",
    "--color-secondary": "#1A1A1A",
    "--color-cta": "#00FF00",
    "--color-background": "#000000",
    "--color-text": "#FFFFFF",
  },
};

// Solarized dark variant
export const SOLARIZED_THEME: Theme = {
  name: "solarized",
  variables: {
    "--color-primary": "#073642",
    "--color-secondary": "#002B36",
    "--color-cta": "#859900",
    "--color-background": "#002B36",
    "--color-text": "#839496",
  },
};

const BUILT_IN_THEMES: Theme[] = [DARK_THEME, LIGHT_THEME, HIGH_CONTRAST_THEME, SOLARIZED_THEME];

export class ThemeManager {
  private current: Theme;
  private customThemes: Theme[] = [];
  private mediaQuery: MediaQueryList | null = null;
  private mediaHandler: ((e: MediaQueryListEvent) => void) | null = null;

  constructor(initial?: Theme) {
    this.current = initial || DARK_THEME;
    this.apply(this.current);
  }

  /** Apply a theme by setting CSS variables on :root */
  apply(theme: Theme): void {
    this.current = theme;
    const root = document.documentElement;
    for (const [key, value] of Object.entries(theme.variables)) {
      root.style.setProperty(key, value);
    }
  }

  /** Switch to a theme by name */
  switchTo(name: string): boolean {
    const theme = this.getAllThemes().find(t => t.name === name);
    if (theme) {
      this.apply(theme);
      return true;
    }
    return false;
  }

  /** Register a custom theme */
  registerTheme(theme: Theme): void {
    this.customThemes.push(theme);
  }

  /** Get all available themes */
  getAllThemes(): Theme[] {
    return [...BUILT_IN_THEMES, ...this.customThemes];
  }

  /** Get current theme */
  getCurrent(): Theme { return this.current; }

  /** Detect system preference and apply */
  applySystemPreference(): void {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    this.apply(prefersDark ? DARK_THEME : LIGHT_THEME);
  }

  /** Listen for system theme changes */
  watchSystemPreference(): void {
    this.mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    this.mediaHandler = (e) => this.apply(e.matches ? DARK_THEME : LIGHT_THEME);
    this.mediaQuery.addEventListener("change", this.mediaHandler);
  }

  destroy(): void {
    if (this.mediaQuery && this.mediaHandler) {
      this.mediaQuery.removeEventListener("change", this.mediaHandler);
    }
  }

  /** Serialize current theme for persistence */
  serialize(): string {
    return JSON.stringify(this.current);
  }

  /** Load theme from serialized string */
  deserialize(json: string): void {
    try {
      const theme: Theme = JSON.parse(json);
      if (theme.name && theme.variables) {
        this.apply(theme);
      }
    } catch { /* ignore invalid JSON */ }
  }
}
