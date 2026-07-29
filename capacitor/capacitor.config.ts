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

  // Desarrollo: cargar desde el servidor local
  // Producción: empaquetar los archivos estáticos
  server: {
    // En desarrollo, permite cargar desde el servidor IAF local
    // cleartext: true permite HTTP (necesario para desarrollo local)
    androidScheme: 'https',
    cleartext: true,
    // Descomentar para desarrollo contra servidor local:
    // url: 'http://192.168.1.X:8080',
    // hostname: 'iaf-local',
  },

  android: {
    allowMixedContent: true,           // Permitir HTTP en WebView
    captureInput: true,                // Capturar input del teclado
    webContentsDebuggingEnabled: true, // Debugging en desarrollo

    // Splash screen
    // splashFullScreen: true,
    // splashImmersive: true,
  },

  plugins: {
    // Plugin de sistema de archivos para operaciones básicas en Android
    Filesystem: {
      // Permite leer/escribir en el almacenamiento interno de la app
    },

    // Plugin de navegador para abrir enlaces externos
    Browser: {
      // Opens external URLs in Chrome Custom Tabs
    },

    // Plugin de red para detectar conectividad
    Network: {
      // Monitorea cambios de conectividad
    },
  },
};

export default config;
