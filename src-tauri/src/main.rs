#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::WebviewBuilder,
    window::{Color, WindowBuilder},
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const HEADER: f64 = 34.0;
const BORDER: f64 = 5.0;
const DW: f64 = 400.0;
const DH: f64 = 640.0;
const MINW: f64 = 230.0;
const MINH: f64 = 160.0;
const COLLAPSED_MINW: f64 = 170.0;
const HOME_LICHESS: &str = "https://lichess.org";
const HOME_CHESS: &str = "https://www.chess.com";

fn home_url(site: &str) -> &'static str {
    if site == "chess" { HOME_CHESS } else { HOME_LICHESS }
}

fn host_ok(h: &str) -> bool {
    h == "lichess.org" || h.ends_with(".lichess.org") || h == "chess.com" || h.ends_with(".chess.com")
}

// Pencere küçülünce (dar/kısa ekran) yan panel, saat, hamle listesi, resign/kontrol
// butonları gibi ögeleri gizleyip sadece satranç tahtasını ortalar.
const COMPACT_JS: &str = r#"(function(){
  function applyCompact(){
    var w = window.innerWidth, h = window.innerHeight;
    var compact = w < 520 || h < 420;
    document.documentElement.classList.toggle('lw-compact', compact);
  }
  window.addEventListener('resize', applyCompact);
  document.addEventListener('DOMContentLoaded', applyCompact);
  applyCompact();
  setInterval(applyCompact, 1000);
  try {
    var mo = new MutationObserver(function(){ applyCompact(); });
    if (document.body) mo.observe(document.body, { childList: true, subtree: false });
    else document.addEventListener('DOMContentLoaded', function(){ mo.observe(document.body, { childList: true, subtree: false }); });
  } catch(e) {}
  var css = [
    'html.lw-compact, html.lw-compact body { overflow:hidden !important; }',
    'html.lw-compact .round__side, html.lw-compact .round__underboard,',
    'html.lw-compact .ricons, html.lw-compact .crosstable, html.lw-compact .context-menu,',
    'html.lw-compact .rclock, html.lw-compact .rmoves, html.lw-compact .control.buttons,',
    'html.lw-compact .game__meta, html.lw-compact .chat__members, html.lw-compact .mchat,',
    'html.lw-compact .continue-with, html.lw-compact .header-wrap, html.lw-compact #top,',
    'html.lw-compact .site-header, html.lw-compact .site-footer,',
    'html.lw-compact .board-layout-sidebar, html.lw-compact .board-layout-chat,',
    'html.lw-compact .board-layout-top, html.lw-compact .board-layout-bottom,',
    'html.lw-compact .game-controls-component, html.lw-compact .move-list-component,',
    'html.lw-compact .players-component, html.lw-compact .navigation-buttons-component,',
    'html.lw-compact .layout-move-list-component, html.lw-compact .board-controls',
    '  { display:none !important; }',
    'html.lw-compact body, html.lw-compact .round__app, html.lw-compact main,',
    'html.lw-compact .board-layout-main, html.lw-compact .board-layout-component',
    '  { display:flex !important; align-items:center !important; justify-content:center !important;',
    '    width:100% !important; height:100% !important; margin:0 !important; padding:0 !important; }',
    'html.lw-compact .cg-wrap, html.lw-compact cg-container, html.lw-compact wc-chess-board,',
    'html.lw-compact .board, html.lw-compact .main-board',
    '  { margin:auto !important; max-width:96vmin !important; max-height:96vmin !important; }'
  ].join('\n');
  var style = document.createElement('style');
  style.setAttribute('data-lw', 'compact');
  style.textContent = css;
  (document.head || document.documentElement).appendChild(style);
})();"#;

fn default_site() -> String {
    "lichess".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
struct St {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    pinned: bool,
    #[serde(default = "default_site")]
    site: String,
    #[serde(skip)]
    collapsed: bool,
}
struct S(Mutex<St>);

#[derive(Serialize)]
struct View {
    pinned: bool,
    collapsed: bool,
    version: String,
    site: String,
}

fn view(app: &AppHandle) -> View {
    let st = app.state::<S>();
    let g = st.0.lock().unwrap();
    View {
        pinned: g.pinned,
        collapsed: g.collapsed,
        version: app.package_info().version.to_string(),
        site: g.site.clone(),
    }
}

fn cfg_path(app: &AppHandle) -> PathBuf {
    app.path().app_config_dir().unwrap_or_default().join("settings.json")
}

fn save(app: &AppHandle) {
    let st = app.state::<S>();
    let mut g = st.0.lock().unwrap();
    if let Some(win) = app.get_window("main") {
        if let (Ok(p), Ok(z), Ok(s)) = (win.outer_position(), win.inner_size(), win.scale_factor()) {
            let p = p.to_logical::<f64>(s);
            let z = z.to_logical::<f64>(s);
            g.x = p.x;
            g.y = p.y;
            if !g.collapsed {
                g.w = z.width;
                g.h = z.height;
            }
        }
    }
    let path = cfg_path(app);
    if let Some(d) = path.parent() {
        let _ = fs::create_dir_all(d);
    }
    if let Ok(j) = serde_json::to_string(&*g) {
        let _ = fs::write(path, j);
    }
}

fn default_state(app: &AppHandle) -> St {
    let (mut x, mut y) = (100.0, 100.0);
    if let Ok(Some(m)) = app.primary_monitor() {
        let s = m.scale_factor();
        x = (m.position().x as f64 + m.size().width as f64) / s - DW - 12.0;
        y = (m.position().y as f64 + m.size().height as f64) / s - DH - 60.0;
    }
    St { x, y, w: DW, h: DH, pinned: true, site: default_site(), collapsed: false }
}

fn on_screen(app: &AppHandle, st: &St) -> bool {
    let Ok(ms) = app.available_monitors() else { return true };
    ms.iter().any(|m| {
        let s = m.scale_factor();
        let (p, z) = (m.position(), m.size());
        let (x, y) = ((st.x * s) as i32 + 40, (st.y * s) as i32 + 20);
        x >= p.x && x < p.x + z.width as i32 && y >= p.y && y < p.y + z.height as i32
    })
}

fn layout(app: &AppHandle) {
    let (Some(win), Some(hd), Some(ch)) =
        (app.get_window("main"), app.get_webview("header"), app.get_webview("lichess"))
    else {
        return;
    };
    let (Ok(sz), Ok(s)) = (win.inner_size(), win.scale_factor()) else { return };
    let b = (BORDER * s).round() as i32;
    let hh = (HEADER * s).round() as i32;
    let (w, h) = (sz.width as i32, sz.height as i32);
    let _ = hd.set_position(PhysicalPosition::new(b, b));
    let _ = hd.set_size(PhysicalSize::new((w - 2 * b).max(1) as u32, (hh - b).max(1) as u32));
    let _ = ch.set_position(PhysicalPosition::new(b, hh));
    let _ = ch.set_size(PhysicalSize::new((w - 2 * b).max(1) as u32, (h - hh - b).max(1) as u32));
}

fn show(app: &AppHandle) {
    if let Some(w) = app.get_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn hide(app: &AppHandle) {
    save(app);
    if let Some(w) = app.get_window("main") {
        let _ = w.hide();
    }
}

// Pencere SADECE Alt+C, ✕ veya tepsi ile gizlenir. Odak kaybında asla gizlenmez.
fn toggle(app: &AppHandle) {
    let visible = app.get_window("main").map(|w| w.is_visible().unwrap_or(false)).unwrap_or(false);
    if visible { hide(app) } else { show(app) }
}

#[tauri::command]
fn get_state(app: AppHandle) -> View {
    view(&app)
}

#[tauri::command]
fn toggle_pin(app: AppHandle) -> View {
    {
        let st = app.state::<S>();
        let mut g = st.0.lock().unwrap();
        g.pinned = !g.pinned;
        if let Some(w) = app.get_window("main") {
            let _ = w.set_always_on_top(g.pinned);
        }
    }
    save(&app);
    view(&app)
}

#[tauri::command]
fn toggle_collapse(app: AppHandle) -> View {
    if let Some(win) = app.get_window("main") {
        let s = win.scale_factor().unwrap_or(1.0);
        let w = win.inner_size().map(|z| z.to_logical::<f64>(s).width).unwrap_or(DW);
        let collapsed = app.state::<S>().0.lock().unwrap().collapsed;
        if !collapsed {
            save(&app);
            let _ = win.set_min_size(Some(LogicalSize::new(COLLAPSED_MINW, HEADER)));
            let _ = win.set_max_size(Some(LogicalSize::new(10000.0, HEADER)));
            let _ = win.set_size(LogicalSize::new(w, HEADER));
            app.state::<S>().0.lock().unwrap().collapsed = true;
        } else {
            app.state::<S>().0.lock().unwrap().collapsed = false;
            let h = app.state::<S>().0.lock().unwrap().h;
            let _ = win.set_max_size(None::<LogicalSize<f64>>);
            let _ = win.set_min_size(Some(LogicalSize::new(MINW, MINH)));
            let _ = win.set_size(LogicalSize::new(w, h));
        }
        layout(&app);
    }
    view(&app)
}

#[tauri::command]
fn hide_widget(app: AppHandle) {
    hide(&app);
}

#[tauri::command]
fn open_lichess(app: AppHandle) {
    let site = app.state::<S>().0.lock().unwrap().site.clone();
    let _ = tauri_plugin_opener::open_url(home_url(&site), None::<&str>);
}

#[tauri::command]
fn toggle_site(app: AppHandle) -> View {
    let new_home;
    {
        let st = app.state::<S>();
        let mut g = st.0.lock().unwrap();
        g.site = if g.site == "chess" { "lichess".to_string() } else { "chess".to_string() };
        new_home = home_url(&g.site);
    }
    if let Some(w) = app.get_webview("lichess") {
        if let Ok(u) = new_home.parse() {
            let _ = w.navigate(u);
        }
    }
    save(&app);
    view(&app)
}

#[tauri::command]
fn start_drag(app: AppHandle) {
    if let Some(w) = app.get_window("main") {
        let _ = w.start_dragging();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _a, _c| show(app)))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _s, e| {
                    if e.state() == ShortcutState::Pressed {
                        toggle(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .invoke_handler(tauri::generate_handler![
            get_state, toggle_pin, toggle_collapse, hide_widget, open_lichess, start_drag, toggle_site
        ])
        .setup(|app| {
            let h = app.handle().clone();
            let hidden = std::env::args().any(|a| a == "--hidden");
            let st = fs::read_to_string(cfg_path(&h))
                .ok()
                .and_then(|s| serde_json::from_str::<St>(&s).ok())
                .filter(|s| s.w >= MINW && s.h >= MINH && on_screen(&h, s))
                .unwrap_or_else(|| default_state(&h));
            app.manage(S(Mutex::new(st.clone())));

            let win = WindowBuilder::new(app, "main")
                .title("Lichess Widget")
                .decorations(false)
                .resizable(true)
                .maximizable(false)
                .skip_taskbar(true)
                .always_on_top(st.pinned)
                .visible(!hidden)
                .background_color(Color(43, 43, 43, 255))
                .inner_size(st.w, st.h)
                .min_inner_size(MINW, MINH)
                .position(st.x, st.y)
                .build()?;

            let header = WebviewBuilder::new("header", WebviewUrl::App("index.html".into()));
            win.add_child(header, LogicalPosition::new(0.0, 0.0), LogicalSize::new(100.0, 30.0))?;

            let lichess = WebviewBuilder::new(
                "lichess",
                WebviewUrl::External(home_url(&st.site).parse().unwrap()),
            )
            .initialization_script(COMPACT_JS)
            .on_navigation(|url| {
                let ok = match url.scheme() {
                    "http" | "https" => url.host_str().map_or(false, host_ok),
                    _ => true,
                };
                if !ok {
                    let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
                }
                ok
            });
            win.add_child(lichess, LogicalPosition::new(0.0, 40.0), LogicalSize::new(100.0, 100.0))?;
            layout(&h);

            let h2 = h.clone();
            win.on_window_event(move |e| match e {
                WindowEvent::Resized(_) => layout(&h2),
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    hide(&h2);
                }
                _ => {}
            });

            let _ = app.global_shortcut().register("Alt+C");

            let t = MenuItem::with_id(app, "toggle", "Göster / Gizle (Alt+C)", true, None::<&str>)?;
            let o = MenuItem::with_id(app, "open", "Lichess.org'u tarayıcıda aç", true, None::<&str>)?;
            let a = CheckMenuItem::with_id(
                app,
                "auto",
                "Windows ile birlikte başlat",
                true,
                h.autolaunch().is_enabled().unwrap_or(false),
                None::<&str>,
            )?;
            let q = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
            let s1 = PredefinedMenuItem::separator(app)?;
            let s2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&t, &o, &s1, &a, &s2, &q])?;
            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;
            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Lichess Widget (Alt+C)")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "toggle" => toggle(app),
                    "open" => {
                        let site = app.state::<S>().0.lock().unwrap().site.clone();
                        let _ = tauri_plugin_opener::open_url(home_url(&site), None::<&str>);
                    }
                    "auto" => {
                        let al = app.autolaunch();
                        if al.is_enabled().unwrap_or(false) {
                            let _ = al.disable();
                        } else {
                            let _ = al.enable();
                        }
                    }
                    "quit" => {
                        save(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = ev
                    {
                        toggle(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("uygulama başlatılamadı");
}
