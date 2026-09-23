// preload.js
'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('widgetAPI', {
  hide: () => ipcRenderer.send('widget:hide'),
  openLichess: () => ipcRenderer.send('widget:open-lichess'),
  togglePin: () => ipcRenderer.send('widget:toggle-pin'),
  getPin: () => ipcRenderer.invoke('widget:get-pin'),
  onPinState: (cb) => ipcRenderer.on('widget:pin-state', (_e, v) => cb(v)),
  toggleCollapse: () => ipcRenderer.send('widget:toggle-collapse'),
  getCollapsed: () => ipcRenderer.invoke('widget:get-collapsed'),
  onCollapseState: (cb) => ipcRenderer.on('widget:collapse-state', (_e, v) => cb(v)),
});
