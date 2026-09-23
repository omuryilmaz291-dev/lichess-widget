// main.js
// Lichess Widget: Alt+C ile açılıp kapanan, boyutu değiştirilebilir, her zaman üstte satranç penceresi.
'use strict';

const {
  app, BrowserWindow, BrowserView, globalShortcut, screen, ipcMain, Tray, Menu, nativeImage, shell,
} = require('electron');
const path = require('path');
const fs = require('fs');

const HEADER_HEIGHT = 34;
const BORDER = 4; // kenarlarda boşluk: boyutlandırma tutamacı + çerçeve görünümü
const DEFAULT_WIDTH = 400;
const DEFAULT_HEIGHT = 640;
const MARGIN = 12;
const LICHESS_HOME = 'https://lichess.org';
const START_HIDDEN = process.argv.includes('--hidden');

let mainWindow = null;
let chessView = null;
let tray = null;
let isVisible = false;
let isAnimating = false;
let settings = { bounds: null, alwaysOnTop: true };
let saveTimer = null;

// ---------------------------------------------------------------------------
// Ayarlar (pencere boyutu/konumu hatırlanır)
// ---------------------------------------------------------------------------
const settingsFile = () => path.join(app.getPath('userData'), 'settings.json');

function loadSettings() {
  try {
    settings = { ...settings, ...JSON.parse(fs.readFileSync(settingsFile(), 'utf8')) };
  } catch (_) { /* ilk çalıştırma */ }
}

function saveSettingsSoon() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    try { fs.writeFileSync(settingsFile(), JSON.stringify(settings)); } catch (_) {}
  }, 400);
}

// ---------------------------------------------------------------------------
// Pencere konumu
// ---------------------------------------------------------------------------
function defaultBounds() {
  const { x, y, width, height } = screen.getPrimaryDisplay().workArea;
  return {
    x: x + width - DEFAULT_WIDTH - MARGIN,
    y: y + height - DEFAULT_HEIGHT - MARGIN,
    width: DEFAULT_WIDTH,
    height: DEFAULT_HEIGHT,
  };
}

// Kayıtlı konum artık hiçbir ekranın içinde değilse (monitör çıkarıldıysa) varsayılana dön
function restoredBounds() {
  const b = settings.bounds;
  if (!b || !Number.isFinite(b.x) || !Number.isFinite(b.width)) return defaultBounds();
  const wa = screen.getDisplayMatching(b).workArea;
  const overlapW = Math.min(b.x + b.width, wa.x + wa.width) - Math.max(b.x, wa.x);
  const overlapH = Math.min(b.y + b.height, wa.y + wa.height) - Math.max(b.y, wa.y);
  return overlapW > 100 && overlapH > 100 ? b : defaultBounds();
}

// ---------------------------------------------------------------------------
// Lichess görünümünü pencerenin içine yerleştir
// ---------------------------------------------------------------------------
function resizeChessView() {
  if (!mainWindow || !chessView) return;
  const { width, height } = mainWindow.getContentBounds();
  chessView.setBounds({
    x: BORDER,
    y: HEADER_HEIGHT,
    width: Math.max(width - BORDER * 2, 0),
    height: Math.max(height - HEADER_HEIGHT - BORDER, 0),
  });
}

// ---------------------------------------------------------------------------
// Açılış / kapanış animasyonu (hafif kayma + opaklık)
// ---------------------------------------------------------------------------
function animateWindow(show) {
  if (isAnimating || !mainWindow) return;
  isAnimating = true;

  const target = restoredBounds();
  const offset = 24;
  const from = show ? { ...target, y: target.y + offset } : { ...target };
  const to = show ? { ...target } : { ...target, y: target.y + offset };

  if (show) {
    mainWindow.setBounds(from);
    mainWindow.setOpacity(0);
    mainWindow.show();
    mainWindow.focus();
  }

  const totalSteps = 13; // ~220ms @ 60fps
  let step = 0;
  const ease = (t) => 1 - Math.pow(1 - t, 3);

  const timer = setInterval(() => {
    step += 1;
    const t = Math.min(step / totalSteps, 1);
    const e = ease(t);
    mainWindow.setBounds({
      x: target.x,
      y: Math.round(from.y + (to.y - from.y) * e),
      width: target.width,
      height: target.height,
    });
    mainWindow.setOpacity(show ? e : 1 - e);

    if (t >= 1) {
      clearInterval(timer);
      isAnimating = false;
      isVisible = show;
      if (!show) {
        mainWindow.hide();
        mainWindow.setBounds(target); // gizliyken gerçek konumu bozulmasın
        mainWindow.setOpacity(1);
      }
      resizeChessView();
    }
  }, 1000 / 60);
}

function showWidget() { if (!isVisible) animateWindow(true); else mainWindow && mainWindow.focus(); }
function hideWidget() { if (isVisible) animateWindow(false); }
function toggleWidget() { isVisible ? hideWidget() : showWidget(); }

// ---------------------------------------------------------------------------
// Ana pencere
// ---------------------------------------------------------------------------
function createWindow() {
  const b = restoredBounds();

  mainWindow = new BrowserWindow({
    ...b,
    minWidth: 300,
    minHeight: 420,
    frame: false,
    resizable: true, // (transparent: true olsaydı Windows'ta boyutlandırma çalışmazdı)
    movable: true,
    skipTaskbar: true,
    alwaysOnTop: settings.alwaysOnTop,
    show: false,
    backgroundColor: '#2b2b2b',
    icon: path.join(__dirname, 'assets', 'icon.png'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  if (settings.alwaysOnTop) mainWindow.setAlwaysOnTop(true, 'screen-saver');

  mainWindow.loadFile(path.join(__dirname, 'renderer', 'index.html'));

  chessView = new BrowserView({
    webPreferences: { contextIsolation: true, nodeIntegration: false },
  });
  mainWindow.setBrowserView(chessView);
  chessView.webContents.loadURL(LICHESS_HOME);

  // lichess.org dışındaki bağlantılar ve yeni sekmeler varsayılan tarayıcıda açılsın
  chessView.webContents.setWindowOpenHandler(({ url }) => {
    if (/^https?:/i.test(url)) shell.openExternal(url);
    return { action: 'deny' };
  });
  chessView.webContents.on('will-navigate', (e, url) => {
    let host = '';
    try { host = new URL(url).hostname; } catch (_) {}
    if (!(host === 'lichess.org' || host.endsWith('.lichess.org'))) {
      e.preventDefault();
      if (/^https?:/i.test(url)) shell.openExternal(url);
    }
  });

  resizeChessView();

  mainWindow.on('resize', () => { resizeChessView(); persistBounds(); });
  mainWindow.on('move', persistBounds);

  // NOT: Widget dışına tıklayınca artık gizlenmiyor. Sadece Alt+C, ✕ butonu veya tepsi ikonu gizler.

  mainWindow.on('closed', () => { mainWindow = null; chessView = null; });
}

function persistBounds() {
  if (!mainWindow || isAnimating || !isVisible) return;
  settings.bounds = mainWindow.getBounds();
  saveSettingsSoon();
}

// ---------------------------------------------------------------------------
// Header butonları
// ---------------------------------------------------------------------------
ipcMain.on('widget:hide', hideWidget);
ipcMain.on('widget:open-lichess', () => shell.openExternal(LICHESS_HOME));
ipcMain.on('widget:toggle-pin', () => {
  if (!mainWindow) return;
  settings.alwaysOnTop = !settings.alwaysOnTop;
  mainWindow.setAlwaysOnTop(settings.alwaysOnTop, settings.alwaysOnTop ? 'screen-saver' : 'normal');
  saveSettingsSoon();
  mainWindow.webContents.send('widget:pin-state', settings.alwaysOnTop);
});
ipcMain.handle('widget:get-pin', () => settings.alwaysOnTop);

// ---------------------------------------------------------------------------
// Sistem tepsisi (saatin yanındaki ^ okunun içinde; sürükleyip dışarı alınabilir)
// ---------------------------------------------------------------------------
function isAutoStartEnabled() {
  return app.getLoginItemSettings().openAtLogin;
}

// Taşınabilir (portable) exe kendini geçici klasöre açar; gerçek exe yolu bu değişkendedir
function realExePath() {
  return process.env.PORTABLE_EXECUTABLE_FILE || process.execPath;
}

function setAutoStart(enabled) {
  app.setLoginItemSettings({
    openAtLogin: enabled,
    path: realExePath(),
    args: ['--hidden'],
  });
}

function buildTrayMenu() {
  return Menu.buildFromTemplate([
    { label: 'Göster / Gizle (Alt+C)', click: toggleWidget },
    { label: 'Lichess.org\'u tarayıcıda aç', click: () => shell.openExternal(LICHESS_HOME) },
    { type: 'separator' },
    {
      label: 'Windows ile birlikte başlat',
      type: 'checkbox',
      checked: isAutoStartEnabled(),
      click: (item) => setAutoStart(item.checked),
    },
    { type: 'separator' },
    { label: 'Çıkış', click: () => app.quit() },
  ]);
}

function createTray() {
  const icon = nativeImage.createFromPath(path.join(__dirname, 'assets', 'icon.png'))
    .resize({ width: 32, height: 32 });
  tray = new Tray(icon);
  tray.setToolTip('Lichess Widget (Alt+C)');
  tray.setContextMenu(buildTrayMenu());
  tray.on('click', toggleWidget);
}

// ---------------------------------------------------------------------------
// Tek örnek kilidi + başlatma
// ---------------------------------------------------------------------------
if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', showWidget); // exe'ye tekrar çift tıklanırsa widget açılır

  app.whenReady().then(() => {
    loadSettings();
    createWindow();
    createTray();
    globalShortcut.register('Alt+C', toggleWidget);
    if (!START_HIDDEN) showWidget(); // ilk açılışta pencere görünür (Windows başlangıcından gizli açılır)
  });

  app.on('window-all-closed', (e) => e.preventDefault());
  app.on('will-quit', () => globalShortcut.unregisterAll());
}
