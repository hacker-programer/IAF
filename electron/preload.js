// ============================================================================
// IAF Electron Client — preload.js
// ============================================================================
//
// Puente seguro entre el renderer (UI web) y el main process (Node.js).
// Expone una API limitada al renderer vía contextBridge.
// ============================================================================

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('iafClient', {
    // Ejecutar una acción localmente (PowerShell, git, archivos, etc.)
    executeLocal: (action, params) => ipcRenderer.invoke('execute-local', action, params),

    // Establecer credenciales después del login en la UI
    setCredentials: (credentials) => ipcRenderer.invoke('set-credentials', credentials),

    // Obtener estado del cliente
    getStatus: () => ipcRenderer.invoke('get-client-status'),

    // Desconectar del servidor
    disconnect: () => ipcRenderer.invoke('disconnect-client'),

    // Escuchar eventos del main process
    onStatusChange: (callback) => {
        ipcRenderer.on('client-status', (event, data) => callback(data));
    },

    // Remover listener
    removeStatusListener: () => {
        ipcRenderer.removeAllListeners('client-status');
    },

    // Verificar si estamos en Electron
    isElectron: true,
});
