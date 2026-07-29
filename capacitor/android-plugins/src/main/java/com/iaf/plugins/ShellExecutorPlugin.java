// ============================================================================
// IAF ShellExecutor Plugin — ShellExecutorPlugin.java
// ============================================================================
//
// Plugin Capacitor nativo para Android que ejecuta comandos shell.
//
// Permite a la app IAF Android ejecutar comandos localmente usando
// Runtime.getRuntime().exec() con el shell del sistema (/system/bin/sh).
//
// Seguridad:
//   - Timeout máximo de 60 segundos
//   - Buffer limitado a 512KB para stdout/stderr
//   - Registro de comandos ejecutados (auditoría)
// ============================================================================

package com.iaf.plugins;

import android.util.Log;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.Map;

@CapacitorPlugin(name = "ShellExecutor")
public class ShellExecutorPlugin extends Plugin {

    private static final String TAG = "IAF-ShellExecutor";
    private static final int MAX_TIMEOUT_SECONDS = 60;
    private static final int DEFAULT_TIMEOUT_SECONDS = 30;
    private static final int MAX_BUFFER_SIZE = 512 * 1024; // 512 KB

    /**
     * Ejecuta un comando shell.
     *
     * Parámetros (JSObject):
     *   - command (String): comando a ejecutar
     *   - timeout (int, opcional): timeout en segundos (default 30, max 60)
     *   - workdir (String, opcional): directorio de trabajo (default /sdcard)
     *   - env (JSObject, opcional): variables de entorno adicionales
     *
     * Respuesta:
     *   - exitCode (int): código de salida
     *   - stdout (String): salida estándar
     *   - stderr (String): salida de error
     *   - elapsedMs (long): tiempo de ejecución
     *   - timedOut (boolean): si fue terminado por timeout
     */
    @PluginMethod
    public void execute(PluginCall call) {
        String command = call.getString("command", "");
        if (command == null || command.trim().isEmpty()) {
            call.reject("El comando no puede estar vacío");
            return;
        }

        int timeout = Math.min(
            call.getInt("timeout", DEFAULT_TIMEOUT_SECONDS),
            MAX_TIMEOUT_SECONDS
        );
        String workdir = call.getString("workdir", "/sdcard");
        JSObject envObj = call.getObject("env");

        Log.i(TAG, "Ejecutando: " + command + " (timeout=" + timeout + "s, workdir=" + workdir + ")");

        long startTime = System.currentTimeMillis();
        boolean timedOut = false;
        int exitCode = -1;
        StringBuilder stdout = new StringBuilder();
        StringBuilder stderr = new StringBuilder();

        Process process = null;
        try {
            ProcessBuilder pb = new ProcessBuilder("/system/bin/sh", "-c", command);
            pb.directory(new File(workdir));

            // Configurar variables de entorno
            Map<String, String> env = pb.environment();
            env.put("HOME", "/sdcard");
            env.put("TMPDIR", "/data/local/tmp");
            if (envObj != null) {
                for (String key : envObj.keys()) {
                    env.put(key, envObj.getString(key, ""));
                }
            }

            // Redirigir stderr al mismo stream que stdout si no existe
            pb.redirectErrorStream(false);

            process = pb.start();

            // Leer stdout y stderr en paralelo
            Thread stdoutThread = new Thread(() -> {
                try (BufferedReader reader = new BufferedReader(
                        new InputStreamReader(process.getInputStream(), "UTF-8"))) {
                    String line;
                    while ((line = reader.readLine()) != null) {
                        if (stdout.length() < MAX_BUFFER_SIZE) {
                            stdout.append(line).append("\n");
                        }
                    }
                } catch (Exception e) {
                    Log.w(TAG, "Error leyendo stdout: " + e.getMessage());
                }
            });

            Thread stderrThread = new Thread(() -> {
                try (BufferedReader reader = new BufferedReader(
                        new InputStreamReader(process.getErrorStream(), "UTF-8"))) {
                    String line;
                    while ((line = reader.readLine()) != null) {
                        if (stderr.length() < MAX_BUFFER_SIZE) {
                            stderr.append(line).append("\n");
                        }
                    }
                } catch (Exception e) {
                    Log.w(TAG, "Error leyendo stderr: " + e.getMessage());
                }
            });

            stdoutThread.start();
            stderrThread.start();

            // Esperar con timeout
            if (process.waitFor(timeout, TimeUnit.SECONDS)) {
                exitCode = process.exitValue();
            } else {
                timedOut = true;
                process.destroyForcibly();
                exitCode = -1;
            }

            // Esperar a que los threads terminen
            stdoutThread.join(2000);
            stderrThread.join(2000);

        } catch (Exception e) {
            Log.e(TAG, "Error ejecutando comando: " + e.getMessage());
            stderr.append("Error: ").append(e.getMessage());
            exitCode = -1;
        } finally {
            if (process != null && process.isAlive()) {
                process.destroyForcibly();
            }
        }

        long elapsedMs = System.currentTimeMillis() - startTime;

        JSObject result = new JSObject();
        result.put("exitCode", exitCode);
        result.put("stdout", trimOutput(stdout.toString()));
        result.put("stderr", trimOutput(stderr.toString()));
        result.put("elapsedMs", elapsedMs);
        result.put("timedOut", timedOut);

        Log.i(TAG, "Comando completado: exitCode=" + exitCode + ", elapsedMs=" + elapsedMs +
            ", timedOut=" + timedOut + ", stdoutLen=" + stdout.length() + ", stderrLen=" + stderr.length());

        call.resolve(result);
    }

    /**
     * Verifica si un comando está disponible en el PATH.
     */
    @PluginMethod
    public void which(PluginCall call) {
        String command = call.getString("command", "");
        if (command == null || command.trim().isEmpty()) {
            call.reject("Comando requerido");
            return;
        }

        // Sanitizar: solo permitir caracteres seguros para which
        if (!command.matches("^[a-zA-Z0-9_-]+$")) {
            JSObject result = new JSObject();
            result.put("found", false);
            result.put("path", (String) null);
            call.resolve(result);
            return;
        }

        try {
            ProcessBuilder pb = new ProcessBuilder("/system/bin/sh", "-c", "which " + command);
            Process process = pb.start();
            BufferedReader reader = new BufferedReader(
                new InputStreamReader(process.getInputStream(), "UTF-8"));
            String path = reader.readLine();
            process.waitFor(5, TimeUnit.SECONDS);

            JSObject result = new JSObject();
            result.put("found", path != null && !path.trim().isEmpty());
            result.put("path", path != null ? path.trim() : null);
            call.resolve(result);
        } catch (Exception e) {
            JSObject result = new JSObject();
            result.put("found", false);
            result.put("path", (String) null);
            call.resolve(result);
        }
    }

    /**
     * Devuelve información del entorno shell.
     */
    @PluginMethod
    public void info(PluginCall call) {
        JSObject result = new JSObject();
        result.put("shell", "/system/bin/sh");

        try {
            result.put("home", System.getenv("HOME") != null ? System.getenv("HOME") : "/sdcard");
            result.put("path", System.getenv("PATH") != null ? System.getenv("PATH") : "/system/bin:/system/xbin");

            // Comandos comúnmente disponibles en Android
            List<String> available = new ArrayList<>();
            String[] commonCommands = {
                "ls", "cat", "echo", "mkdir", "rm", "cp", "mv", "pwd",
                "chmod", "ps", "df", "du", "grep", "find", "head", "tail",
                "wc", "sort", "uniq", "cut", "tr", "sed", "awk",
                "curl", "wget", "tar", "gzip", "date", "whoami", "id", "uname"
            };

            ProcessBuilder pb = new ProcessBuilder("/system/bin/sh", "-c",
                "which " + String.join(" ", commonCommands));
            Process process = pb.start();
            BufferedReader reader = new BufferedReader(
                new InputStreamReader(process.getInputStream(), "UTF-8"));
            String line;
            while ((line = reader.readLine()) != null) {
                String cmd = line.trim();
                if (!cmd.isEmpty()) {
                    // Extraer el nombre del comando del path
                    String cmdName = cmd.substring(cmd.lastIndexOf('/') + 1);
                    available.add(cmdName);
                }
            }
            process.waitFor(5, TimeUnit.SECONDS);

            result.put("availableCommands", available.toArray(new String[0]));
        } catch (Exception e) {
            result.put("availableCommands", new String[0]);
        }

        call.resolve(result);
    }

    /** Trimea output y limita tamaño máximo */
    private String trimOutput(String s) {
        if (s == null) return "";
        s = s.trim();
        if (s.length() > MAX_BUFFER_SIZE) {
            return s.substring(0, MAX_BUFFER_SIZE) + "\n... [TRUNCADO]";
        }
        return s;
    }
}
