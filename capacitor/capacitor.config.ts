// ============================================================================
// IAF Capacitor Config — capacitor.config.ts
// ============================================================================
//
// Configuración de Capacitor para la app Android de IAF.
//
// Arquitectura:
//   - La UI web existente (public/) se empaqueta dentro de la app Android
//   - En modo "cliente conectado": la app web se conecta al servidor IAF
//     y el cliente Electron en la PC del usuario ejecuta comandos localmente
//   - En modo "admin": el servidor ejecuta comandos directamente
//   - Para operaciones básicas de archivos: usa el plugin @capacitor/filesystem
//
// Build:
//   cd capacitor
//   npm install
//   npm run init        # inicializa el proyecto Capacitor
//   npm run add-android # agrega la plataforma Android
//   npm run sync        # sincroniza assets web con Android
//   npm run open        # abre en Android Studio
// ============================================================================

import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
  appId: 'com.iaf.app',
  appName: 'IAF',
  webDir: '../public',       // La UI web existente

  server: {
    // FIX #33: En producción usar HTTPS. cleartext solo en desarrollo local.
    androidScheme: 'https',
    // cleartext: true se deshabilita en producción. Solo activar para
    // desarrollo contra servidor local sin HTTPS.
    // cleartext: true,
  },

  android: {
    allowMixedContent: false,          // FIX #33: Bloquear HTTP mixto en producción
    captureInput: true,
    webContentsDebuggingEnabled: false, // Deshabilitar en producción
  },

  plugins: {
    Filesystem: {},
    Browser: {},
    Network: {},
  },
};

export default config;
