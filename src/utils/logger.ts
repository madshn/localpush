type LogLevel = 'debug' | 'info' | 'warn' | 'error';

const LOG_LEVELS: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };

function _timestamp(): string {
  return new Date().toISOString();
}

class Logger {
  private level: LogLevel = 'info';

  private shouldLog(level: LogLevel): boolean {
    return LOG_LEVELS[level] >= LOG_LEVELS[this.level];
  }

  debug(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('debug'))
      // biome-ignore lint/suspicious/noConsole: intentional logger utility
      console.debug(`[${_timestamp()}] DEBUG:`, msg, data ?? '');
  }
  info(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('info'))
      // biome-ignore lint/suspicious/noConsole: intentional logger utility
      console.info(`[${_timestamp()}] INFO:`, msg, data ?? '');
  }
  warn(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('warn'))
      // biome-ignore lint/suspicious/noConsole: intentional logger utility
      console.warn(`[${_timestamp()}] WARN:`, msg, data ?? '');
  }
  error(msg: string, data?: Record<string, unknown>) {
    if (this.shouldLog('error'))
      // biome-ignore lint/suspicious/noConsole: intentional logger utility
      console.error(`[${_timestamp()}] ERROR:`, msg, data ?? '');
  }
}

export const logger = new Logger();
