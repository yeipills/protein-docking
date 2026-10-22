/*!
 * Theme Store - Gestión de temas y preferencias visuales
 */

import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';
import type { Writable, Readable } from 'svelte/store';

export type Theme = 'dark' | 'light' | 'auto';
export type ColorScheme = 'blue' | 'green' | 'purple' | 'orange';

export interface ThemeConfig {
  theme: Theme;
  colorScheme: ColorScheme;
  reducedMotion: boolean;
  highContrast: boolean;
  fontSize: 'small' | 'medium' | 'large';
}

export interface ThemeColors {
  primary: string;
  secondary: string;
  accent: string;
  background: string;
  surface: string;
  text: string;
  textSecondary: string;
  border: string;
  success: string;
  warning: string;
  error: string;
}

class ThemeStore {
  private config: Writable<ThemeConfig>;
  private systemTheme: Writable<'dark' | 'light'>;

  constructor() {
    // Configuración por defecto
    this.config = writable<ThemeConfig>({
      theme: 'auto',
      colorScheme: 'green',
      reducedMotion: false,
      highContrast: false,
      fontSize: 'medium'
    });

    // Tema del sistema
    this.systemTheme = writable<'dark' | 'light'>('dark');

    if (browser) {
      this.initializeBrowserTheme();
    }
  }

  get store(): Writable<ThemeConfig> {
    return this.config;
  }

  // Tema actual resuelto
  get resolvedTheme(): Readable<'dark' | 'light'> {
    return derived(
      [this.config, this.systemTheme],
      ([$config, $systemTheme]) => {
        if ($config.theme === 'auto') {
          return $systemTheme;
        }
        return $config.theme as 'dark' | 'light';
      }
    );
  }

  // Colores del tema actual
  get colors(): Readable<ThemeColors> {
    return derived(
      [this.resolvedTheme, this.config],
      ([$theme, $config]) => {
        return this.getThemeColors($theme, $config.colorScheme, $config.highContrast);
      }
    );
  }

  // CSS variables derivadas
  get cssVariables(): Readable<Record<string, string>> {
    return derived(this.colors, ($colors) => {
      return Object.entries($colors).reduce((vars, [key, value]) => {
        vars[`--color-${key.replace(/([A-Z])/g, '-$1').toLowerCase()}`] = value;
        return vars;
      }, {} as Record<string, string>);
    });
  }

  // Inicializar tema en el browser
  private initializeBrowserTheme(): void {
    // Cargar configuración guardada
    this.loadSavedConfig();

    // Detectar tema del sistema
    this.detectSystemTheme();

    // Escuchar cambios del sistema
    this.listenToSystemChanges();

    // Detectar preferencias de accesibilidad
    this.detectAccessibilityPreferences();

    // Aplicar tema inicial
    this.applyTheme();

    // Suscribirse a cambios para aplicar automáticamente
    this.config.subscribe(() => {
      this.applyTheme();
      this.saveConfig();
    });
  }

  // Cargar configuración guardada
  private loadSavedConfig(): void {
    try {
      const saved = localStorage.getItem('theme-config');
      if (saved) {
        const config = JSON.parse(saved);
        this.config.set({ ...this.getDefaultConfig(), ...config });
      }
    } catch (error) {
      console.warn('Failed to load theme config:', error);
    }
  }

  // Guardar configuración
  private saveConfig(): void {
    try {
      this.config.subscribe(config => {
        localStorage.setItem('theme-config', JSON.stringify(config));
      })();
    } catch (error) {
      console.warn('Failed to save theme config:', error);
    }
  }

  // Detectar tema del sistema
  private detectSystemTheme(): void {
    if (window.matchMedia) {
      const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      this.systemTheme.set(isDark ? 'dark' : 'light');
    }
  }

  // Escuchar cambios del tema del sistema
  private listenToSystemChanges(): void {
    if (window.matchMedia) {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      mediaQuery.addEventListener('change', (e) => {
        this.systemTheme.set(e.matches ? 'dark' : 'light');
      });
    }
  }

  // Detectar preferencias de accesibilidad
  private detectAccessibilityPreferences(): void {
    if (window.matchMedia) {
      // Reduced motion
      const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      
      // High contrast
      const prefersHighContrast = window.matchMedia('(prefers-contrast: high)').matches;

      this.config.update(config => ({
        ...config,
        reducedMotion: prefersReducedMotion,
        highContrast: prefersHighContrast
      }));
    }
  }

  // Aplicar tema al DOM
  private applyTheme(): void {
    let currentConfig: ThemeConfig;
    let currentTheme: 'dark' | 'light';
    let currentColors: ThemeColors;
    let currentVars: Record<string, string>;

    this.config.subscribe(c => currentConfig = c)();
    this.resolvedTheme.subscribe(t => currentTheme = t)();
    this.colors.subscribe(c => currentColors = c)();
    this.cssVariables.subscribe(v => currentVars = v)();

    // Aplicar clase de tema
    document.documentElement.className = `theme-${currentTheme}`;
    
    // Aplicar esquema de colores
    document.documentElement.setAttribute('data-color-scheme', currentConfig!.colorScheme);
    
    // Aplicar variables CSS
    Object.entries(currentVars!).forEach(([property, value]) => {
      document.documentElement.style.setProperty(property, value);
    });

    // Aplicar preferencias de accesibilidad
    document.documentElement.style.setProperty(
      '--animation-duration',
      currentConfig!.reducedMotion ? '0ms' : '300ms'
    );

    document.documentElement.style.setProperty(
      '--font-size-base',
      this.getFontSizeValue(currentConfig!.fontSize)
    );

    // Meta theme-color para navegadores móviles
    let themeColorMeta = document.querySelector('meta[name="theme-color"]');
    if (!themeColorMeta) {
      themeColorMeta = document.createElement('meta');
      themeColorMeta.setAttribute('name', 'theme-color');
      document.head.appendChild(themeColorMeta);
    }
    themeColorMeta.setAttribute('content', currentColors!.background);
  }

  // Obtener colores del tema
  private getThemeColors(theme: 'dark' | 'light', scheme: ColorScheme, highContrast: boolean): ThemeColors {
    const colorSchemes = {
      blue: {
        primary: '#007AFF',
        secondary: '#5856D6',
        accent: '#64D2FF'
      },
      green: {
        primary: '#00FF88',
        secondary: '#00CC66',
        accent: '#88FFBB'
      },
      purple: {
        primary: '#8A2BE2',
        secondary: '#9932CC',
        accent: '#DDA0DD'
      },
      orange: {
        primary: '#FF9500',
        secondary: '#FF6B35',
        accent: '#FFB366'
      }
    };

    const colors = colorSchemes[scheme];
    
    if (theme === 'dark') {
      return {
        ...colors,
        background: highContrast ? '#000000' : '#1a1a1a',
        surface: highContrast ? '#111111' : '#2d2d2d',
        text: highContrast ? '#ffffff' : '#ffffff',
        textSecondary: highContrast ? '#cccccc' : '#888888',
        border: highContrast ? '#444444' : '#333333',
        success: '#00FF88',
        warning: '#FFD60A',
        error: '#FF453A'
      };
    } else {
      return {
        ...colors,
        background: highContrast ? '#ffffff' : '#f5f5f5',
        surface: highContrast ? '#f0f0f0' : '#ffffff',
        text: highContrast ? '#000000' : '#1a1a1a',
        textSecondary: highContrast ? '#333333' : '#666666',
        border: highContrast ? '#888888' : '#e0e0e0',
        success: '#00C853',
        warning: '#FF8F00',
        error: '#D32F2F'
      };
    }
  }

  // Obtener valor de font-size
  private getFontSizeValue(size: 'small' | 'medium' | 'large'): string {
    switch (size) {
      case 'small': return '14px';
      case 'medium': return '16px';
      case 'large': return '18px';
      default: return '16px';
    }
  }

  // Configuración por defecto
  private getDefaultConfig(): ThemeConfig {
    return {
      theme: 'auto',
      colorScheme: 'green',
      reducedMotion: false,
      highContrast: false,
      fontSize: 'medium'
    };
  }

  // Métodos públicos para cambiar configuración
  setTheme(theme: Theme): void {
    this.config.update(config => ({ ...config, theme }));
  }

  setColorScheme(colorScheme: ColorScheme): void {
    this.config.update(config => ({ ...config, colorScheme }));
  }

  setFontSize(fontSize: 'small' | 'medium' | 'large'): void {
    this.config.update(config => ({ ...config, fontSize }));
  }

  toggleHighContrast(): void {
    this.config.update(config => ({ ...config, highContrast: !config.highContrast }));
  }

  resetToDefaults(): void {
    this.config.set(this.getDefaultConfig());
  }
}

// Singleton instance
export const themeStore = new ThemeStore();