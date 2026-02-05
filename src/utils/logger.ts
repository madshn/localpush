type LogLevel = 'debug' | 'info' | 'warn' | 'error';

const LOG_LEVELS: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };

class Logger {
  private level: LogLevel = 'debug';

  private shouldLog(level: LogLevel): boolean {
    return LOG_LEVELS[level] >= LOG_LEVELS[this.level];
  }

  debug(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('debug')) console.debug(`[LocalPush] ${msg}`, data ?? '');
  }
  info(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('info')) console.info(`[LocalPush] ${msg}`, data ?? '');
  }
  warn(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('warn')) console.warn(`[LocalPush] ${msg}`, data ?? '');
  }
  error(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('error')) console.error(`[LocalPush] ${msg}`, data ?? '');
  }
}

export const logger = new Logger();
