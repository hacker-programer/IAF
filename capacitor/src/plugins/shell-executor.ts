// ============================================================================
// IAF ShellExecutor Plugin — shell-executor.ts
// ============================================================================
//
// Plugin Capacitor personalizado para ejecutar comandos shell en Android.
// Permite que la app Android de IAF ejecute comandos localmente (ls, cat, grep,
// find, curl, etc.) sin depender de un cliente Electron en PC.
//
// Limitaciones en Android:
//   - NO tiene PowerShell. Usa /system/bin/sh (shell POSIX básico).
//   - NO tiene cargo, git, rustc (a menos que se instalen via Termux).
//   - Comandos disponibles: ls, cat, grep, find, curl, wget, echo, mkdir, rm, cp, mv, ps, df...
//   - Para funcionalidad completa (cargo, git), instalar Termux y agregar ~/bin al PATH.
//
// Uso desde JavaScript:
//   import { ShellExecutor } from './plugins/shell-executor';
//   const result = await ShellExecutor.execute({ command: 'ls -la /sdcard' });
//
// Seguridad:
//   - Solo ejecuta comandos si el usuario está autenticado
//   - Timeout máximo de 60 segundos por comando
//   - Sanitización básica contra inyección de comandos
// ============================================================================

import { registerPlugin } from '@capacitor/core';

export interface ShellExecuteOptions {
  /** Comando shell completo a ejecutar (ej: "ls -la /sdcard") */
  command: string;
  /** Timeout en segundos (default: 30, max: 60) */
  timeout?: number;
  /** Directorio de trabajo (default: /sdcard) */
  workdir?: string;
  /** Variables de entorno adicionales */
  env?: Record<string, string>;
}

export interface ShellExecuteResult {
  /** Código de salida (0 = éxito) */
  exitCode: number;
  /** Salida estándar (stdout) */
  stdout: string;
  /** Salida de error (stderr) */
  stderr: string;
  /** Tiempo de ejecución en ms */
  elapsedMs: number;
  /** Si el comando fue terminado por timeout */
  timedOut: boolean;
}

export interface ShellExecutorPlugin {
  execute(options: ShellExecuteOptions): Promise<ShellExecuteResult>;
  /** Verifica si un comando está disponible en el sistema */
  which(options: { command: string }): Promise<{ found: boolean; path?: string }>;
  /** Devuelve información del entorno shell */
  info(): Promise<{
    shell: string;
    home: string;
    path: string;
    availableCommands: string[];
  }>;
}

const ShellExecutor = registerPlugin<ShellExecutorPlugin>('ShellExecutor');

export { ShellExecutor };
