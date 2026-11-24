#![windows_subsystem = "windows"] // no console window

// StarPlay: mpv renders through Direct3D 11 into a Win32 window (the "host"); the Slint UI is a
// transparent frameless window, owned by the host and aligned to its client area.

mod preview;
mod ui;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use libmpv2::{Mpv, events::Event};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::event::{ElementState, WindowEvent};
use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, ModelRc, VecModel};
use preview::Preview;
use ui::{Overlay, Recent, StripItem, Track};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute};
use windows_sys::Win32::UI::Controls::MARGINS;
use windows_sys::Win32::Graphics::Gdi::{
    BLACK_BRUSH, ClientToScreen, GetMonitorInfoW, GetStockObject, MONITOR_DEFAULTTONEAREST,
    MONITORINFO, MonitorFromWindow,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, GetSystemMetricsForDpi,
    SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static HOST: AtomicIsize = AtomicIsize::new(0);
static OVERLAY: AtomicIsize = AtomicIsize::new(0);
static MPV: Mutex<Option<Arc<Mpv>>> = Mutex::new(None);
static PREVIEW: OnceLock<Preview> = OnceLock::new();
/// Video frame rate (fps), to convert a time into a frame number.
static FPS: Mutex<f64> = Mutex::new(25.0);
static SAVED_PLACEMENT: Mutex<Option<WINDOWPLACEMENT>> = Mutex::new(None);
/// Margins of the video image inside the window (black bars), in pixels: left, top, right, bottom.
static VIDEO_MARGINS: Mutex<[f64; 4]> = Mutex::new([0.0; 4]);

// Requests from the UI to the host, posted so as not to start a modal loop or resize
// the Slint window while winit is handling one of its events.
const WM_APP_FULLSCREEN: u32 = WM_APP + 1;
const WM_APP_OPEN: u32 = WM_APP + 2;
const WM_APP_DRAG: u32 = WM_APP + 3; // wparam = HT* hit-test code (move / border)
const WM_APP_MINIMIZE: u32 = WM_APP + 4;
const WM_APP_MAXIMIZE: u32 = WM_APP + 5;

const HIDE_DELAY: Duration = Duration::from_millis(2500);
const SPEEDS: [f64; 9] = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0];
const MAX_RECENT: usize = 10;

thread_local! {
    static UI: RefCell<Option<Overlay>> = const { RefCell::new(None) };
    static HIDE_TIMER: slint::Timer = slint::Timer::default();
    // Thumbnails already decoded by the preview engine (frame index -> image), cleared for each file
    static THUMBS: RefCell<HashMap<i64, slint::Image>> = RefCell::new(HashMap::new());
    static HOVER_FRAME: Cell<i64> = const { Cell::new(-1) };
    static STRIP_CENTER: Cell<Option<i64>> = const { Cell::new(None) };
    // Fine scrub: was playback running before (to resume it on release)?
    static RESUME_AFTER_SCRUB: Cell<bool> = const { Cell::new(false) };
}

fn with_ui(f: impl FnOnce(&Overlay)) {
    UI.with(|u| {
        if let Ok(u) = u.try_borrow()
            && let Some(ui) = u.as_ref()
        {
            f(ui)
        }
    });
}

fn with_mpv(f: impl FnOnce(&Mpv)) {
    if let Some(mpv) = MPV.lock().unwrap().as_ref() {
        f(mpv)
    }
}

fn mpv_cmd(name: &str, args: &[&str]) {
    with_mpv(|m| {
        let _ = m.command(name, args);
    });
}

fn post_host(msg: u32, wp: usize) {
    unsafe { PostMessageW(HOST.load(Ordering::Relaxed), msg, wp, 0) };
}

/// Shows the controls and restarts the hide delay (kept while the mouse is over the panel,
/// a menu is open, or the video is paused).
fn poke() {
    with_ui(|ui| ui.set_controls_visible(true));
    HIDE_TIMER.with(|t| {
        t.start(slint::TimerMode::SingleShot, HIDE_DELAY, || {
            let mut keep = false;
            with_ui(|ui| {
                keep = ui.get_panel_hovered() || ui.get_paused() || ui.get_verify();
                if !keep {
                    ui.set_controls_visible(false);
                }
            });
            if keep {
                poke();
            }
        })
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Before any window: otherwise Windows would stretch the host on a 150% display (blurry image).
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

    let args: Vec<String> = std::env::args().skip(1).collect();
    let file = args.iter().find(|a| !a.starts_with("--")).cloned();

    let host = create_host_window();
    HOST.store(host, Ordering::Relaxed);

    let mpv = Arc::new(Mpv::with_initializer(|init| {
        init.set_option("wid", host as i64)?;
        init.set_option("vo", "gpu-next")?;
        init.set_option("gpu-api", "d3d11")?;
        init.set_option("target-colorspace-hint", "yes")?; // switches output to HDR10 if the display is in HDR
        init.set_option("hwdec", "auto-safe")?;
        init.set_option("keep-open", "yes")?;
        init.set_option("osd-level", 0i64)?; // the Slint UI replaces mpv's OSD
        init.set_option("input-vo-keyboard", "no")?; // keys go to the host, not to the mpv window
        // Liquid glass under the controls, computed by mpv on the image (copied next to the exe by build.rs)
        let glass = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.join("glass.glsl")));
        if let Some(g) = glass.filter(|g| g.exists()) {
            init.set_option("glsl-shaders", g.to_string_lossy().as_ref())?;
        }
        // Any --key=value command-line option is passed to mpv as is.
        for opt in args.iter().filter_map(|a| a.strip_prefix("--")) {
            let (k, v) = opt.split_once('=').unwrap_or((opt, "yes"));
            init.set_option(k, v)?;
        }
        Ok(())
    })?);
    *MPV.lock().unwrap() = Some(mpv.clone());
    let _ = PREVIEW.set(Preview::spawn(|n, buf| {
        let _ = slint::invoke_from_event_loop(move || on_thumbnail(n, slint::Image::from_rgba8(buf)));
    }));
    if let Some(f) = &file {
        open_path(f); // right away: with --idle=no, mpv would stop without a file
    }

    slint::BackendSelector::new()
        .backend_name("winit".into())
        .with_winit_window_attributes_hook(move |a| {
            a.with_owner_window(host).with_skip_taskbar(true).with_resizable(false)
        })
        .select()?;

    let ui = Overlay::new()?;
    ui.on_activity(poke);
    ui.on_toggle_pause(|| mpv_cmd("cycle", &["pause"]));
    ui.on_toggle_fullscreen(|| post_host(WM_APP_FULLSCREEN, 0));
    ui.on_seek(|t, exact| {
        if exact {
            seek_frame((t as f64 * fps()).round() as i64);
        } else {
            mpv_cmd("seek", &[&t.to_string(), "absolute+keyframes"]); // while dragging: keyframe, immediately
        }
    });
    ui.on_hover_time(|_, n| {
        let n = n as i64;
        HOVER_FRAME.set(n);
        match THUMBS.with(|t| t.borrow().get(&n).cloned()) {
            Some(img) => with_ui(|ui| ui.set_preview(img)),
            None => request_thumbs([n]),
        }
    });
    ui.on_precise_start(|| {
        let mut playing = false;
        with_mpv(|m| {
            playing = !m.get_property::<bool>("pause").unwrap_or(true);
            if playing {
                let _ = m.set_property("pause", true);
            }
        });
        RESUME_AFTER_SCRUB.set(playing);
        // The mouse stays inside the window during fine scrub (cursor hidden, endless scrolling)
        let r = client_rect_on_screen();
        unsafe { ClipCursor(&r) };
    });
    ui.on_precise_frame(|n, done| {
        if STRIP_CENTER.get() != Some(n as i64) || done {
            seek_frame(n as i64);
        }
        if done {
            unsafe { ClipCursor(std::ptr::null()) };
        } else {
            wrap_cursor();
        }
        if done && RESUME_AFTER_SCRUB.replace(false) {
            with_mpv(|m| {
                let _ = m.set_property("pause", false);
            });
        }
        STRIP_CENTER.set((!done).then_some(n as i64));
        if !done {
            // the target frame first, then its neighbours farther and farther out, two ahead on each side
            let n = n as i64;
            let r = strip_radius() + 2;
            request_thumbs(std::iter::once(n).chain((1..=r).flat_map(|d| [n - d, n + d])));
        }
        update_strip();
    });
    ui.on_step_frames(step_frames);
    ui.on_begin_edit(begin_edit);
    ui.on_skip(|s| mpv_cmd("seek", &[&s.to_string(), "relative+exact"]));
    ui.on_set_volume(|v| {
        with_mpv(|m| {
            let _ = m.set_property("volume", v.round() as f64);
            let _ = m.set_property("mute", false);
        })
    });
    ui.on_volume_step(volume_step);
    ui.on_toggle_mute(|| mpv_cmd("cycle", &["mute"]));
    ui.on_set_speed(|s| with_mpv(|m| {
        let _ = m.set_property("speed", s as f64);
    }));
    ui.on_select_audio(|id| with_mpv(|m| {
        let _ = m.set_property("aid", id as i64);
    }));
    ui.on_select_sub(|id| with_mpv(|m| {
        let _ = if id == 0 { m.set_property("sid", "no") } else { m.set_property("sid", id as i64) };
    }));
    ui.on_open_dialog(|| post_host(WM_APP_OPEN, 0));
    ui.on_open_path(|p| open_path(p.as_str()));
    ui.on_window_drag(|ht| post_host(WM_APP_DRAG, ht as usize));
    ui.on_minimize(|| post_host(WM_APP_MINIMIZE, 0));
    ui.on_maximize(|| post_host(WM_APP_MAXIMIZE, 0));
    ui.on_close_window(|| post_host(WM_CLOSE, 0));
    ui.set_recent(recent_model(&load_recent()));
    ui.show()?;
    UI.with(|u| *u.borrow_mut() = Some(ui.clone_strong()));

    slint::invoke_from_event_loop(|| {
        with_ui(|ui| {
            // Drag and drop: winit receives the file on the UI window, which covers the video.
            ui.window().on_winit_window_event(|_, ev| match ev {
                WindowEvent::DroppedFile(p) => {
                    open_path(&p.to_string_lossy());
                    EventResult::PreventDefault
                }
                // The UI never activates (WS_EX_NOACTIVATE): a click in it must still bring StarPlay
                // to the front, otherwise keys (shortcuts, typing) go to the previous app.
                WindowEvent::MouseInput { state: ElementState::Pressed, .. } => {
                    let host = HOST.load(Ordering::Relaxed);
                    unsafe {
                        if GetForegroundWindow() != host {
                            SetForegroundWindow(host);
                        }
                    }
                    EventResult::Propagate
                }
                _ => EventResult::Propagate,
            });
            let ov = ui.window().with_winit_window(|w| match w.window_handle().map(|h| h.as_raw()) {
                Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
                _ => 0,
            });
            if let Some(ov) = ov.filter(|&h| h != 0) {
                unsafe {
                    // Never active: the host window keeps keyboard focus and its active title bar.
                    let ex = GetWindowLongW(ov, GWL_EXSTYLE);
                    SetWindowLongW(ov, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE as i32);
                }
                OVERLAY.store(ov, Ordering::Relaxed);
                sync_overlay(HOST.load(Ordering::Relaxed));
            }
        });
        poke();
    })?;

    // The glass follows the panel and the menu (position, size, fade); mpv is only called when it changes.
    let glass_timer = slint::Timer::default();
    {
        let mut last = String::new();
        glass_timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
            with_ui(|ui| {
                let s = ui.window().scale_factor();
                let [ox, oy, mr, mb] = *VIDEO_MARGINS.lock().unwrap();
                ui.set_video_left((ox / s as f64) as f32);
                ui.set_video_top((oy / s as f64) as f32);
                ui.set_video_right((mr / s as f64) as f32);
                ui.set_video_bottom((mb / s as f64) as f32);
                let opts = format!(
                    concat!("origin_x={},origin_y={},scale={},",
                    "panel_x={},panel_y={},panel_w={},panel_h={},panel_r={},panel_a={:.3},",
                    "menu_x={},menu_y={},menu_w={},menu_h={},menu_r={},menu_a={:.3},",
                    "win_x={},win_y={},win_d={},win_step={},win_a={:.3},",
                    "mode_x={},mode_y={},mode_w={},mode_h={},mode_r={},mode_a={:.3}"),
                    ox, oy, s,
                    ui.get_panel_px() * s, ui.get_panel_py() * s, ui.get_panel_pw() * s,
                    ui.get_panel_ph() * s, ui.get_radius_panel() * s, ui.get_panel_alpha(),
                    ui.get_menu_px() * s, ui.get_menu_py() * s, ui.get_menu_pw() * s,
                    ui.get_menu_ph() * s, ui.get_radius_menu() * s, ui.get_menu_alpha(),
                    ui.get_top_px() * s, ui.get_top_py() * s, ui.get_win_d() * s,
                    ui.get_win_step() * s, ui.get_top_alpha(),
                    ui.get_mode_px() * s, ui.get_mode_py() * s, ui.get_mode_pw() * s,
                    ui.get_mode_ph() * s, ui.get_mode_ph() * s / 2.0, ui.get_mode_alpha(),
                );
                if opts != last {
                    with_mpv(|m| {
                        let _ = m.set_property("glsl-shader-opts", opts.as_str());
                    });
                    last = opts;
                }
            });
        });
    }

    // mpv event thread: reads the state and pushes it to the UI without ever waiting for it.
    let last_stats = Arc::new(Mutex::new(String::new()));
    let events = {
        let (mpv, weak, last_stats) = (mpv.clone(), ui.as_weak(), last_stats.clone());
        std::thread::spawn(move || {
            let mut slow_at = Instant::now() - Duration::from_secs(1);
            let (mut title, mut tracks) = (String::new(), (Vec::new(), Vec::new()));
            loop {
                if let Some(Ok(Event::Shutdown)) = mpv.wait_event(0.1) {
                    break;
                }
                let b = |p: &str| mpv.get_property::<bool>(p).unwrap_or(false);
                let f = |p: &str| mpv.get_property::<f64>(p).unwrap_or(0.0);
                let (pos, dur, vol, speed) = (f("time-pos"), f("duration"), f("volume"), f("speed"));
                let (paused, muted, has_video) = (b("pause"), b("mute"), !b("idle-active"));
                let rate = [f("container-fps"), f("estimated-vf-fps")].into_iter().find(|r| *r > 0.0).unwrap_or(25.0);
                *FPS.lock().unwrap() = rate;
                let (frame, total) = ((pos * rate).round() as i32, (dur * rate).round() as i32);
                *VIDEO_MARGINS.lock().unwrap() =
                    ["ml", "mt", "mr", "mb"].map(|k| f(&format!("osd-dimensions/{k}")));

                // Every 500 ms: stats, title, tracks (sent only when changed).
                let (mut new_title, mut new_tracks, mut stats) = (None, None, None);
                if slow_at.elapsed() >= Duration::from_millis(500) {
                    slow_at = Instant::now();
                    let s = read_stats(&mpv);
                    if dur > 0.0 {
                        *last_stats.lock().unwrap() = s.clone(); // not the empty reading after the end
                    }
                    stats = Some(s);
                    let t = if has_video { mpv.get_property::<String>("media-title").unwrap_or_default() } else { String::new() };
                    if t != title {
                        title = t.clone();
                        new_title = Some(t);
                    }
                    let tr = read_tracks(&mpv);
                    if tr != tracks {
                        tracks = tr.clone();
                        new_tracks = Some(tr);
                    }
                }
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_position(pos as f32);
                    ui.set_duration(dur as f32);
                    ui.set_paused(paused);
                    ui.set_volume(vol as f32);
                    ui.set_muted(muted);
                    ui.set_speed(speed as f32);
                    ui.set_has_video(has_video);
                    ui.set_fps(rate as f32);
                    ui.set_frame(frame);
                    ui.set_total_frames(total);
                    if let Some(s) = stats {
                        ui.set_stats(s.into());
                    }
                    if let Some(t) = new_title {
                        let text = if t.is_empty() { "StarPlay".to_string() } else { format!("{t} — StarPlay") };
                        let w: Vec<u16> = text.encode_utf16().chain([0]).collect();
                        unsafe { SetWindowTextW(HOST.load(Ordering::Relaxed), w.as_ptr()) };
                    }
                    if let Some((audio, subs)) = new_tracks {
                        ui.set_audio_tracks(track_model(audio));
                        ui.set_sub_tracks(track_model(subs));
                    }
                });
            }
            // mpv stopped on its own (--idle=no at end of file, quit command...)
            let _ = slint::invoke_from_event_loop(|| {
                let _ = slint::quit_event_loop();
            });
        })
    };

    slint::run_event_loop_until_quit()?;

    let _ = mpv.command("quit", &[]);
    let _ = events.join();
    // Final reading (lost frames...) for measurements: STARPLAY_STATS=<file>
    if let Some(f) = std::env::var_os("STARPLAY_STATS") {
        let _ = std::fs::write(f, last_stats.lock().unwrap().as_bytes());
    }
    UI.with(|u| u.borrow_mut().take());
    MPV.lock().unwrap().take();
    drop(mpv); // last handle: mpv releases its window before the host is destroyed
    unsafe { DestroyWindow(host) };
    Ok(())
}

type TrackList = Vec<(i64, String, bool)>;

/// Audio and subtitle tracks; the subtitle list starts with "Off" (id 0).
fn read_tracks(mpv: &Mpv) -> (TrackList, TrackList) {
    let (mut audio, mut subs) = (Vec::new(), vec![(0, "Off".to_string(), true)]);
    let n = mpv.get_property::<i64>("track-list/count").unwrap_or(0);
    for i in 0..n {
        let s = |k: &str| mpv.get_property::<String>(&format!("track-list/{i}/{k}")).ok();
        let id = mpv.get_property::<i64>(&format!("track-list/{i}/id")).unwrap_or(0);
        let selected = mpv.get_property::<bool>(&format!("track-list/{i}/selected")).unwrap_or(false);
        let label = match (s("title"), s("lang")) {
            (Some(t), Some(l)) => format!("{t} ({l})"),
            (Some(t), None) => t,
            (None, Some(l)) => l,
            (None, None) => format!("Track {id}"),
        };
        match s("type").as_deref() {
            Some("audio") => audio.push((id, label, selected)),
            Some("sub") => {
                if selected {
                    subs[0].2 = false;
                }
                subs.push((id, label, selected));
            }
            _ => {}
        }
    }
    (audio, subs)
}

fn track_model(list: TrackList) -> ModelRc<Track> {
    let v: Vec<Track> = list
        .into_iter()
        .map(|(id, label, selected)| Track { id: id as i32, label: label.into(), selected })
        .collect();
    ModelRc::new(VecModel::from(v))
}

fn fps() -> f64 {
    *FPS.lock().unwrap()
}

/// Time to seek to in order to show frame n.
fn frame_time(n: i64) -> f64 {
    n.max(0) as f64 / fps()
}

/// Exact seek (not the nearest keyframe) to frame n.
fn seek_frame(n: i64) {
    mpv_cmd("seek", &[&frame_time(n).to_string(), "absolute+exact"]);
}

fn step_frames(d: i32) {
    mpv_cmd(if d > 0 { "frame-step" } else { "frame-back-step" }, &[]);
}

/// Thumbnail arriving from the preview engine: cached, and shown if expected (hover, frame strip).
fn on_thumbnail(n: i64, img: slint::Image) {
    THUMBS.with(|t| {
        let mut t = t.borrow_mut();
        if t.len() > 400 {
            t.clear(); // ponytail: simple cache; a real LRU if we keep thousands of thumbnails
        }
        t.insert(n, img.clone());
    });
    if n == HOVER_FRAME.get() {
        with_ui(|ui| ui.set_preview(img));
    }
    if STRIP_CENTER.get().is_some_and(|c| (c - n).abs() <= strip_radius()) {
        update_strip();
    }
}

/// Asks the preview engine for frames not cached yet (in the given order).
fn request_thumbs(frames: impl IntoIterator<Item = i64>) {
    let mut total = 0;
    with_ui(|ui| total = ui.get_total_frames() as i64);
    let missing: Vec<(f64, i64)> = THUMBS.with(|t| {
        let t = t.borrow();
        frames.into_iter().filter(|n| (0..total).contains(n) && !t.contains_key(n)).map(|n| (frame_time(n), n)).collect()
    });
    if let Some(p) = PREVIEW.get() {
        p.request(missing);
    }
}

/// Fine-scrub strip: the target frame plus as many neighbours on each side as the screen width allows.
fn strip_radius() -> i64 {
    let mut r = 2;
    with_ui(|ui| r = ui.get_strip_radius() as i64);
    r
}

fn update_strip() {
    with_ui(|ui| {
        let items: Vec<StripItem> = match STRIP_CENTER.get() {
            None => Vec::new(),
            Some(c) => (c - strip_radius()..=c + strip_radius())
                .filter(|n| (0..ui.get_total_frames() as i64).contains(n))
                .map(|n| {
                    let img = THUMBS.with(|t| t.borrow().get(&n).cloned());
                    StripItem { frame: n as i32, ready: img.is_some(), image: img.unwrap_or_default(), current: n == c }
                })
                .collect(),
        };
        ui.set_strip_center(STRIP_CENTER.get().unwrap_or(0) as i32);
        ui.set_strip(ModelRc::new(VecModel::from(items)));
    });
}

/// Opens an input field: 1 = go to (frame or timecode), 2 = speed.
fn begin_edit(field: i32) {
    with_ui(|ui| {
        ui.set_edit_text("".into());
        ui.set_edit_field(field);
    });
}

/// "Go to" field: a frame number counted from 1 (1234) or a timecode (HH:MM:SS:FF, MM:SS:FF, SS:FF;
/// ";" and "." also accepted). Returns the frame index (from 0).
fn parse_goto(text: &str, fps: f64) -> Option<i64> {
    let parts: Vec<&str> = text.trim().split([':', ';', '.']).collect();
    if parts.len() > 4 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    if parts.len() == 1 {
        return Some((parts[0].parse::<i64>().ok()? - 1).max(0));
    }
    let f = fps.round().max(1.0) as i64;
    let units = [1, f, 60 * f, 3600 * f];
    parts.iter().rev().zip(units).try_fold(0i64, |n, (p, u)| Some(n + p.parse::<i64>().ok()? * u))
}

/// Typed speed: "1.5", "1,5", "0.25x"; clamped to mpv's limits.
fn parse_speed(text: &str) -> Option<f64> {
    let v: f64 = text.trim().trim_end_matches(['x', '×']).replace(',', ".").parse().ok()?;
    (v > 0.0).then(|| v.clamp(0.01, 100.0))
}

/// Input for the active field (keys reach the host: the UI never takes focus).
fn on_edit_char(c: char) {
    with_ui(|ui| {
        let mut text = ui.get_edit_text().to_string();
        match c {
            '\r' => {
                let field = ui.get_edit_field();
                ui.set_edit_field(0);
                if field == 1
                    && let Some(n) = parse_goto(&text, fps())
                {
                    seek_frame(n.min((ui.get_total_frames() as i64 - 1).max(0)));
                }
                if field == 2
                    && let Some(v) = parse_speed(&text)
                {
                    with_mpv(|m| {
                        let _ = m.set_property("speed", v);
                    });
                }
                return;
            }
            '\u{1b}' => {
                ui.set_edit_field(0);
                return;
            }
            '\u{8}' => {
                text.pop();
            }
            '0'..='9' | ':' | ';' | '.' | ',' if text.len() < 16 => text.push(c),
            _ => {}
        }
        ui.set_edit_text(text.into());
    });
}

fn volume_step(d: f32) {
    with_mpv(|m| {
        let _ = m.command("add", &["volume", &d.to_string()]);
        let _ = m.set_property("mute", false);
    });
}

fn speed_step(up: bool) {
    with_mpv(|m| {
        let cur = m.get_property::<f64>("speed").unwrap_or(1.0);
        let next = if up {
            SPEEDS.iter().find(|&&s| s > cur + 1e-6)
        } else {
            SPEEDS.iter().rev().find(|&&s| s < cur - 1e-6)
        };
        if let Some(&s) = next {
            let _ = m.set_property("speed", s);
        }
    });
}

fn open_path(path: &str) {
    mpv_cmd("loadfile", &[path]);
    if let Some(p) = PREVIEW.get() {
        p.open(path);
    }
    THUMBS.with(|t| t.borrow_mut().clear());
    // same file, same entry: Windows paths with "\" (the command line may give "/")
    let path = path.replace('/', "\\");
    let mut list = load_recent();
    list.retain(|p| p.replace('/', "\\") != path);
    list.insert(0, path);
    list.truncate(MAX_RECENT);
    save_recent(&list);
    with_ui(|ui| ui.set_recent(recent_model(&list)));
}

// ponytail: recent list as plain text; to merge into the settings file of #5.
fn recent_file() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("StarPlay").join("recent.txt"))
}

fn load_recent() -> Vec<String> {
    recent_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default()
}

fn save_recent(list: &[String]) {
    if let Some(f) = recent_file() {
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let _ = std::fs::write(f, list.join("\n"));
    }
}

fn recent_model(list: &[String]) -> ModelRc<Recent> {
    let v: Vec<Recent> = list
        .iter()
        .map(|p| {
            let path = Path::new(p);
            Recent {
                path: p.into(),
                name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().into(),
                folder: path.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default().into(),
            }
        })
        .collect();
    ModelRc::new(VecModel::from(v))
}

fn open_dialog(owner: HWND) {
    let mut buf = [0u16; 4096];
    let filter: Vec<u16> =
        "Videos\0*.mov;*.mp4;*.m4v;*.mkv;*.webm;*.avi;*.mxf;*.ts;*.m2ts;*.wmv;*.flv;*.mpg;*.mpeg\0All files\0*.*\0\0"
            .encode_utf16()
            .collect();
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = buf.as_mut_ptr();
    ofn.nMaxFile = buf.len() as u32;
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;
    if unsafe { GetOpenFileNameW(&mut ofn) } != 0 {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
        open_path(&String::from_utf16_lossy(&buf[..len]));
    }
}

fn read_stats(mpv: &Mpv) -> String {
    let s = |p: &str| mpv.get_property::<String>(p).unwrap_or_else(|_| "-".into());
    let i = |p: &str| mpv.get_property::<i64>(p).unwrap_or(-1);
    let f = |p: &str| mpv.get_property::<f64>(p).unwrap_or(0.0);
    format!(
        "{} {}x{} {:.3} fps | decoding: {} | vo: {}\n\
         source: {} / {} ({})  ->  output: {} / {}\n\
         display {:.2} Hz | frame {} | dropped vo {}  decoder {}  delayed {}  mistimed {}",
        s("video-codec"), i("width"), i("height"), f("container-fps"),
        s("hwdec-current"), s("current-vo"),
        s("video-params/primaries"), s("video-params/gamma"), s("video-params/pixelformat"),
        s("video-target-params/primaries"), s("video-target-params/gamma"),
        f("display-fps"), i("estimated-frame-number"),
        i("frame-drop-count"), i("decoder-frame-drop-count"),
        i("vo-delayed-frame-count"), i("mistimed-frame-count"),
    )
}

fn create_host_window() -> HWND {
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class: Vec<u16> = "StarPlayVideo\0".encode_utf16().collect();
        let title: Vec<u16> = "StarPlay\0".encode_utf16().collect();
        // App icon (resource 1 of the exe, see starplay.rc): taskbar, Alt+Tab
        let icon = |size: i32| LoadImageW(instance, 1 as _, IMAGE_ICON, size, size, LR_DEFAULTCOLOR);
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(host_proc),
            hInstance: instance,
            hIcon: icon(GetSystemMetrics(SM_CXICON)),
            hIconSm: icon(GetSystemMetrics(SM_CXSMICON)),
            hCursor: LoadCursorW(0, IDC_ARROW),
            hbrBackground: GetStockObject(BLACK_BRUSH),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            0, class.as_ptr(), title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT, 1280, 760,
            0, 0, instance, std::ptr::null(),
        );
        assert!(hwnd != 0, "CreateWindowExW failed");
        // No title bar (WM_NCCALCSIZE) but keeps the shadow, rounded corners and Windows snapping
        let dark: i32 = 1; // DWMWA_USE_IMMERSIVE_DARK_MODE: dark border
        DwmSetWindowAttribute(hwnd, 20, &dark as *const i32 as _, 4);
        let round: i32 = 2; // DWMWA_WINDOW_CORNER_PREFERENCE = rounded (Windows 11)
        DwmSetWindowAttribute(hwnd, 33, &round as *const i32 as _, 4);
        let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 1, cyBottomHeight: 0 };
        DwmExtendFrameIntoClientArea(hwnd, &margins); // keeps the window shadow
        SetWindowPos(hwnd, 0, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
        hwnd
    }
}

/// Shortcuts taken from mpv's, read as characters so they also work on AZERTY layouts.
fn on_char(hwnd: HWND, c: char) -> bool {
    let mut editing = false;
    let mut verify = false;
    with_ui(|ui| {
        editing = ui.get_edit_field() != 0;
        verify = ui.get_verify();
    });
    if editing {
        on_edit_char(c);
        return true;
    }
    match c {
        '\t' => with_ui(|ui| ui.set_verify(!ui.get_verify())), // toggle player / verify mode
        'g' if verify => begin_edit(1),
        ' ' | 'p' => mpv_cmd("cycle", &["pause"]),
        'f' | 'F' => toggle_fullscreen(hwnd),
        '\u{1b}' if SAVED_PLACEMENT.lock().unwrap().is_some() => toggle_fullscreen(hwnd),
        'm' => mpv_cmd("cycle", &["mute"]),
        '9' | '/' => volume_step(-2.0),
        '0' | '*' => volume_step(2.0),
        '[' => speed_step(false),
        ']' => speed_step(true),
        '\u{8}' => with_mpv(|m| {
            let _ = m.set_property("speed", 1.0);
        }),
        ',' => mpv_cmd("frame-back-step", &[]),
        '.' => mpv_cmd("frame-step", &[]),
        'j' => mpv_cmd("cycle", &["sub"]),
        '#' => mpv_cmd("cycle", &["audio"]),
        'i' => with_ui(|ui| ui.set_show_stats(!ui.get_show_stats())),
        'q' => unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        },
        '\u{f}' => open_dialog(hwnd), // Ctrl+O
        _ => return false,
    }
    true
}

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_WINDOWPOSCHANGED => sync_overlay(hwnd),
        WM_KEYDOWN => {
            let mut verify = false;
            with_ui(|ui| verify = ui.get_verify() && ui.get_edit_field() == 0);
            let shift = unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
            // Verify mode: ← / → = one frame, Shift + ← / → = one second
            if verify && matches!(wp as u16, VK_LEFT | VK_RIGHT) {
                let forward = wp as u16 == VK_RIGHT;
                if shift {
                    mpv_cmd("seek", &[if forward { "1" } else { "-1" }, "relative+exact"]);
                } else {
                    step_frames(if forward { 1 } else { -1 });
                }
                return 0;
            }
            let seek = match wp as u16 {
                VK_LEFT => "-5",
                VK_RIGHT => "5",
                VK_UP => "60",
                VK_DOWN => "-60",
                _ => "",
            };
            if !seek.is_empty() {
                mpv_cmd("seek", &[seek, "relative+exact"]);
                poke();
                return 0;
            }
        }
        WM_CHAR => {
            if char::from_u32(wp as u32).is_some_and(|c| on_char(hwnd, c)) {
                poke();
                return 0;
            }
        }
        WM_NCCALCSIZE if wp != 0 => {
            // The whole window is client area; when maximized, Windows overflows the frame: we remove it.
            if unsafe { IsZoomed(hwnd) } != 0 {
                let dpi = unsafe { GetDpiForWindow(hwnd) };
                let b = unsafe {
                    GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
                };
                let r = unsafe { &mut (*(lp as *mut NCCALCSIZE_PARAMS)).rgrc[0] };
                r.left += b;
                r.top += b;
                r.right -= b;
                r.bottom -= b;
            }
            return 0;
        }
        WM_SIZE => with_ui(|ui| ui.set_is_maximized(wp as u32 == SIZE_MAXIMIZED)),
        // Safety net: StarPlay loses the foreground during a fine scrub -> the mouse is no longer locked
        WM_ACTIVATE if (wp & 0xFFFF) as u32 == WA_INACTIVE => unsafe {
            ClipCursor(std::ptr::null());
        },
        WM_APP_DRAG => unsafe {
            // Native move / resize, started from the UI that covers the window
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            ReleaseCapture();
            SendMessageW(hwnd, WM_NCLBUTTONDOWN, wp, ((pt.y as u32 as isize) << 16) | (pt.x as u16 as isize));
            // The modal loop swallowed the release: we give it back to the UI so it does not stay "pressed"
            PostMessageW(OVERLAY.load(Ordering::Relaxed), WM_LBUTTONUP, 0, 0);
        },
        WM_APP_MINIMIZE => unsafe {
            ShowWindow(hwnd, SW_MINIMIZE);
        },
        WM_APP_MAXIMIZE => unsafe {
            ShowWindow(hwnd, if IsZoomed(hwnd) != 0 { SW_RESTORE } else { SW_MAXIMIZE });
        },
        WM_APP_FULLSCREEN => toggle_fullscreen(hwnd),
        WM_APP_OPEN => open_dialog(hwnd),
        WM_CLOSE => {
            // The window is only destroyed after mpv has stopped (end of main).
            let _ = slint::quit_event_loop();
            return 0;
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

/// Client area of the host window, in screen coordinates.
fn client_rect_on_screen() -> RECT {
    let host = HOST.load(Ordering::Relaxed);
    unsafe {
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(host, &mut rc);
        let mut pt = POINT { x: 0, y: 0 };
        ClientToScreen(host, &mut pt);
        RECT { left: pt.x, top: pt.y, right: pt.x + rc.right, bottom: pt.y + rc.bottom }
    }
}

/// Fine scrub: when the (hidden) mouse nears an edge, move it back to the centre and shift the UI's
/// reference point by the same amount, so the strip keeps scrolling without a jump.
fn wrap_cursor() {
    let r = client_rect_on_screen();
    let mut pt = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut pt) };
    let margin = 40.min((r.right - r.left) / 4);
    if pt.x > r.left + margin && pt.x < r.right - margin {
        return;
    }
    let center = (r.left + r.right) / 2;
    unsafe { SetCursorPos(center, pt.y) };
    with_ui(|ui| {
        let dx = (center - pt.x) as f32 / ui.window().scale_factor();
        ui.set_last_x(ui.get_last_x() + dx);
    });
}

/// Aligns the UI on the host's client area (physical pixels).
fn sync_overlay(host: HWND) {
    let ov = OVERLAY.load(Ordering::Relaxed);
    if ov == 0 || host == 0 {
        return;
    }
    unsafe {
        if IsIconic(host) != 0 {
            return;
        }
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(host, &mut rc);
        let mut pt = POINT { x: 0, y: 0 };
        ClientToScreen(host, &mut pt);
        SetWindowPos(ov, 0, pt.x, pt.y, rc.right, rc.bottom,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER);
    }
}

/// "Borderless windowed" fullscreen on the window's monitor (Raymond Chen's method).
fn toggle_fullscreen(hwnd: HWND) {
    let mut saved = SAVED_PLACEMENT.lock().unwrap();
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_STYLE);
        match saved.take() {
            None => {
                let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
                wp.length = size_of::<WINDOWPLACEMENT>() as u32;
                let mut mi: MONITORINFO = std::mem::zeroed();
                mi.cbSize = size_of::<MONITORINFO>() as u32;
                if GetWindowPlacement(hwnd, &mut wp) != 0
                    && GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut mi) != 0
                {
                    *saved = Some(wp);
                    with_ui(|ui| ui.set_is_fullscreen(true));
                    SetWindowLongW(hwnd, GWL_STYLE, style & !(WS_OVERLAPPEDWINDOW as i32));
                    let r = mi.rcMonitor;
                    // Without touching the Z order: HWND_TOP + SWP_NOOWNERZORDER put the video in front of the UI.
                    SetWindowPos(hwnd, 0, r.left, r.top, r.right - r.left, r.bottom - r.top,
                        SWP_NOZORDER | SWP_FRAMECHANGED);
                }
            }
            Some(wp) => {
                with_ui(|ui| ui.set_is_fullscreen(false));
                SetWindowLongW(hwnd, GWL_STYLE, style | WS_OVERLAPPEDWINDOW as i32);
                SetWindowPlacement(hwnd, &wp);
                SetWindowPos(hwnd, 0, 0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_goto, parse_speed};

    #[test]
    fn speed_text() {
        assert_eq!(parse_speed("1.5"), Some(1.5));
        assert_eq!(parse_speed("0,25x"), Some(0.25));
        assert_eq!(parse_speed("500"), Some(100.0));
        assert_eq!(parse_speed("0"), None);
        assert_eq!(parse_speed(""), None);
    }

    #[test]
    fn goto_frame_and_timecode() {
        assert_eq!(parse_goto("1234", 25.0), Some(1233)); // frames counted from 1
        assert_eq!(parse_goto("1", 25.0), Some(0));
        assert_eq!(parse_goto("0", 25.0), Some(0));
        assert_eq!(parse_goto("00:00:01:05", 25.0), Some(30));
        assert_eq!(parse_goto("1:00:00:00", 24.0), Some(86400));
        assert_eq!(parse_goto("2:10", 30.0), Some(70)); // SS:FF
        assert_eq!(parse_goto("01:02:03", 29.97), Some(1800 + 2 * 30 + 3)); // MM:SS:FF, nominal 30 fps
        assert_eq!(parse_goto("", 25.0), None);
        assert_eq!(parse_goto("1::2", 25.0), None);
        assert_eq!(parse_goto("1:2:3:4:5", 25.0), None);
    }
}

