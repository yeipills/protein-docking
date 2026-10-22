/*!
 * Logger - Sistema de logging centralizado para Protein Docking v2
 * 
 * Características:
 * 1. Múltiples niveles de log (DEBUG, INFO, WARN, ERROR)
 * 2. Logging estructurado con contexto
 * 3. Persistencia local con rotación de logs
 * 4. Filtros configurables
 * 5. Exportación de logs para debugging
 * 6. Performance tracking
 * 7. Error reporting con stack traces
 */

export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

export interface LogEntry {
  id: string;
  timestamp: Date;
  level: LogLevel;
  message: string;
  context?: string;
  module?: string;
  data?: any;
  error?: Error;
  stackTrace?: string;
  userId?: string;
  sessionId: string;
  performance?: {
    duration?: number;
    memory?: number;
    operation?: string;
  };
}

export interface LoggerConfig {
  level: LogLevel;
  enableConsole: boolean;
  enableStorage: boolean;
  maxStorageEntries: number;
  enablePerformanceTracking: boolean;
  enableStackTraces: boolean;
  modules: string[];
  blacklistModules: string[];
}

export interface LogFilter {
  level?: LogLevel[];
  module?: string[];
  timeRange?: {
    start: Date;
    end: Date;
  };
  searchTerm?: string;
}

export interface LogStatistics {
  totalEntries: number;
  byLevel: Record<LogLevel, number>;
  byModule: Record<string, number>;
  errors: LogEntry[];
  warnings: LogEntry[];
  averageMemoryUsage: number;
  sessionDuration: number;
}

class Logger {
  private config: LoggerConfig;
  private logs: LogEntry[];
  private sessionId: string;
  private sessionStartTime: Date;
  private performanceMarks: Map<string, number>;

  constructor(config?: Partial<LoggerConfig>) {
    this.config = {
      level: 'INFO',
      enableConsole: true,
      enableStorage: true,
      maxStorageEntries: 1000,
      enablePerformanceTracking: true,
      enableStackTraces: false,
      modules: [],
      blacklistModules: [],
      ...config
    };

    this.logs = [];
    this.sessionId = this.generateSessionId();
    this.sessionStartTime = new Date();
    this.performanceMarks = new Map();

    this.loadStoredLogs();
    this.log('INFO', 'Logger initialized', 'Logger', { config: this.config });
  }

  /**
   * Registra un mensaje de debug
   */
  debug(message: string, module?: string, data?: any): void {
    this.log('DEBUG', message, module, data);
  }

  /**
   * Registra un mensaje informativo
   */
  info(message: string, module?: string, data?: any): void {
    this.log('INFO', message, module, data);
  }

  /**
   * Registra una advertencia
   */
  warn(message: string, module?: string, data?: any): void {
    this.log('WARN', message, module, data);
  }

  /**
   * Registra un error
   */
  error(message: string, module?: string, error?: Error, data?: any): void {
    this.log('ERROR', message, module, data, error);
  }

  /**
   * Inicia el tracking de performance para una operación
   */
  startPerformanceTracking(operation: string): void {
    if (!this.config.enablePerformanceTracking) return;

    this.performanceMarks.set(operation, performance.now());
    this.debug(`Started performance tracking: ${operation}`, 'Performance');
  }

  /**
   * Finaliza el tracking de performance y registra el resultado
   */
  endPerformanceTracking(operation: string, module?: string, data?: any): number {
    if (!this.config.enablePerformanceTracking) return 0;

    const startTime = this.performanceMarks.get(operation);
    if (!startTime) {
      this.warn(`No start time found for operation: ${operation}`, 'Performance');
      return 0;
    }

    const duration = performance.now() - startTime;
    this.performanceMarks.delete(operation);

    const memoryInfo = this.getMemoryInfo();
    
    this.log('INFO', `Performance: ${operation} completed`, module || 'Performance', {
      ...data,
      performance: {
        duration: Math.round(duration * 100) / 100,
        memory: memoryInfo.usedJSHeapSize,
        operation
      }
    });

    return duration;
  }

  /**
   * Registra una entrada de log
   */
  private log(level: LogLevel, message: string, module?: string, data?: any, error?: Error): void {
    // Verificar nivel de log
    if (!this.shouldLog(level)) return;

    // Verificar filtros de módulo
    if (module && !this.shouldLogModule(module)) return;

    const logEntry: LogEntry = {
      id: this.generateLogId(),
      timestamp: new Date(),
      level,
      message,
      context: this.getContext(),
      module,
      data,
      error,
      stackTrace: this.config.enableStackTraces && error ? error.stack : undefined,
      sessionId: this.sessionId,
      performance: data?.performance
    };

    // Agregar a la cola de logs
    this.logs.push(logEntry);

    // Mantener límite de logs en memoria
    if (this.logs.length > this.config.maxStorageEntries) {
      this.logs.shift();
    }

    // Log a consola
    if (this.config.enableConsole) {
      this.logToConsole(logEntry);
    }

    // Persistir si está habilitado
    if (this.config.enableStorage) {
      this.persistLogs();
    }
  }

  /**
   * Registra en la consola del navegador
   */
  private logToConsole(entry: LogEntry): void {
    const prefix = `[${entry.timestamp.toISOString()}] [${entry.level}] ${entry.module ? `[${entry.module}]` : ''}`;
    const message = `${prefix} ${entry.message}`;

    switch (entry.level) {
      case 'DEBUG':
        console.debug(message, entry.data || '');
        break;
      case 'INFO':
        console.info(message, entry.data || '');
        break;
      case 'WARN':
        console.warn(message, entry.data || '');
        break;
      case 'ERROR':
        console.error(message, entry.error || entry.data || '');
        if (entry.stackTrace) {
          console.error(entry.stackTrace);
        }
        break;
    }
  }

  /**
   * Verifica si se debe registrar un nivel de log
   */
  private shouldLog(level: LogLevel): boolean {
    const levels: LogLevel[] = ['DEBUG', 'INFO', 'WARN', 'ERROR'];
    const currentLevelIndex = levels.indexOf(this.config.level);
    const logLevelIndex = levels.indexOf(level);
    return logLevelIndex >= currentLevelIndex;
  }

  /**
   * Verifica si se debe registrar un módulo
   */
  private shouldLogModule(module: string): boolean {
    // Si hay blacklist y el módulo está en ella, no registrar
    if (this.config.blacklistModules.length > 0 && this.config.blacklistModules.includes(module)) {
      return false;
    }

    // Si hay whitelist y el módulo no está en ella, no registrar
    if (this.config.modules.length > 0 && !this.config.modules.includes(module)) {
      return false;
    }

    return true;
  }

  /**
   * Obtiene el contexto actual
   */
  private getContext(): string {
    if (typeof window !== 'undefined') {
      return `${window.location.pathname}${window.location.search}`;
    }
    return 'unknown';
  }

  /**
   * Obtiene información de memoria
   */
  private getMemoryInfo(): any {
    if (typeof window !== 'undefined' && 'performance' in window && 'memory' in (window.performance as any)) {
      return (window.performance as any).memory;
    }
    return { usedJSHeapSize: 0, totalJSHeapSize: 0, jsHeapSizeLimit: 0 };
  }

  /**
   * Genera un ID único para la sesión
   */
  private generateSessionId(): string {
    return `session_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Genera un ID único para el log
   */
  private generateLogId(): string {
    return `log_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`;
  }

  /**
   * Carga logs almacenados desde localStorage
   */
  private loadStoredLogs(): void {
    if (typeof window === 'undefined') return;

    try {
      const stored = localStorage.getItem('protein_docking_logs');
      if (stored) {
        const parsedLogs = JSON.parse(stored).map((log: any) => ({
          ...log,
          timestamp: new Date(log.timestamp)
        }));
        this.logs = parsedLogs.slice(-this.config.maxStorageEntries);
      }
    } catch (error) {
      console.warn('Failed to load stored logs:', error);
    }
  }

  /**
   * Persiste logs en localStorage
   */
  private persistLogs(): void {
    if (typeof window === 'undefined') return;

    try {
      // Solo persistir los últimos logs para evitar problemas de espacio
      const logsToStore = this.logs.slice(-Math.min(this.config.maxStorageEntries, 500));
      localStorage.setItem('protein_docking_logs', JSON.stringify(logsToStore));
    } catch (error) {
      console.warn('Failed to persist logs:', error);
      // Si falla por espacio, limpiar logs antiguos
      this.clearOldLogs();
    }
  }

  /**
   * Limpia logs antiguos para liberar espacio
   */
  private clearOldLogs(): void {
    const keepCount = Math.floor(this.config.maxStorageEntries / 2);
    this.logs = this.logs.slice(-keepCount);
    
    try {
      localStorage.setItem('protein_docking_logs', JSON.stringify(this.logs));
    } catch (error) {
      console.warn('Failed to clear old logs:', error);
    }
  }

  /**
   * Obtiene todos los logs
   */
  getLogs(): LogEntry[] {
    return [...this.logs];
  }

  /**
   * Filtra logs según criterios
   */
  filterLogs(filter: LogFilter): LogEntry[] {
    return this.logs.filter(log => {
      // Filtrar por nivel
      if (filter.level && !filter.level.includes(log.level)) {
        return false;
      }

      // Filtrar por módulo
      if (filter.module && filter.module.length > 0) {
        if (!log.module || !filter.module.includes(log.module)) {
          return false;
        }
      }

      // Filtrar por rango de tiempo
      if (filter.timeRange) {
        if (log.timestamp < filter.timeRange.start || log.timestamp > filter.timeRange.end) {
          return false;
        }
      }

      // Filtrar por término de búsqueda
      if (filter.searchTerm) {
        const searchLower = filter.searchTerm.toLowerCase();
        const messageMatch = log.message.toLowerCase().includes(searchLower);
        const moduleMatch = log.module?.toLowerCase().includes(searchLower);
        const dataMatch = log.data && JSON.stringify(log.data).toLowerCase().includes(searchLower);
        
        if (!messageMatch && !moduleMatch && !dataMatch) {
          return false;
        }
      }

      return true;
    });
  }

  /**
   * Obtiene estadísticas de logs
   */
  getStatistics(): LogStatistics {
    const byLevel: Record<LogLevel, number> = {
      DEBUG: 0,
      INFO: 0,
      WARN: 0,
      ERROR: 0
    };

    const byModule: Record<string, number> = {};
    const errors: LogEntry[] = [];
    const warnings: LogEntry[] = [];
    let totalMemory = 0;
    let memoryCount = 0;

    this.logs.forEach(log => {
      // Contar por nivel
      byLevel[log.level]++;

      // Contar por módulo
      if (log.module) {
        byModule[log.module] = (byModule[log.module] || 0) + 1;
      }

      // Recopilar errores y advertencias
      if (log.level === 'ERROR') {
        errors.push(log);
      } else if (log.level === 'WARN') {
        warnings.push(log);
      }

      // Calcular memoria promedio
      if (log.performance?.memory) {
        totalMemory += log.performance.memory;
        memoryCount++;
      }
    });

    const sessionDuration = Date.now() - this.sessionStartTime.getTime();

    return {
      totalEntries: this.logs.length,
      byLevel,
      byModule,
      errors: errors.slice(-10), // Últimos 10 errores
      warnings: warnings.slice(-10), // Últimas 10 advertencias
      averageMemoryUsage: memoryCount > 0 ? totalMemory / memoryCount : 0,
      sessionDuration
    };
  }

  /**
   * Exporta logs en formato JSON
   */
  exportLogs(filter?: LogFilter): string {
    const logsToExport = filter ? this.filterLogs(filter) : this.logs;
    const exportData = {
      metadata: {
        exportDate: new Date().toISOString(),
        sessionId: this.sessionId,
        totalEntries: logsToExport.length,
        config: this.config
      },
      logs: logsToExport
    };

    return JSON.stringify(exportData, null, 2);
  }

  /**
   * Limpia todos los logs
   */
  clearLogs(): void {
    this.logs = [];
    if (typeof window !== 'undefined') {
      localStorage.removeItem('protein_docking_logs');
    }
    this.info('Logs cleared', 'Logger');
  }

  /**
   * Actualiza la configuración del logger
   */
  updateConfig(newConfig: Partial<LoggerConfig>): void {
    this.config = { ...this.config, ...newConfig };
    this.info('Logger configuration updated', 'Logger', { config: this.config });
  }

  /**
   * Obtiene la configuración actual
   */
  getConfig(): LoggerConfig {
    return { ...this.config };
  }

  /**
   * Obtiene información de la sesión
   */
  getSessionInfo(): { sessionId: string; startTime: Date; duration: number } {
    return {
      sessionId: this.sessionId,
      startTime: this.sessionStartTime,
      duration: Date.now() - this.sessionStartTime.getTime()
    };
  }
}

// Instancia singleton del logger
export const logger = new Logger({
  level: 'INFO',
  enableConsole: true,
  enableStorage: true,
  maxStorageEntries: 1000,
  enablePerformanceTracking: true,
  enableStackTraces: false
});

// Funciones de conveniencia
export const log = {
  debug: (message: string, module?: string, data?: any) => logger.debug(message, module, data),
  info: (message: string, module?: string, data?: any) => logger.info(message, module, data),
  warn: (message: string, module?: string, data?: any) => logger.warn(message, module, data),
  error: (message: string, module?: string, error?: Error, data?: any) => logger.error(message, module, error, data),
  startPerf: (operation: string) => logger.startPerformanceTracking(operation),
  endPerf: (operation: string, module?: string, data?: any) => logger.endPerformanceTracking(operation, module, data)
};

export default logger;