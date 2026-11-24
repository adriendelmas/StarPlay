// Hover preview on the timeline (#7): a second mpv engine, with no display and no sound, seeks to the exact hovered frame,
// scales it down to 320 px and renders it to the UI. The main engine is never touched.
// The frame is drawn by libmpv's software renderer ("sw" API) into our memory: unlike
// screenshot-raw with vo=null, which rendered the previous request's frame while paused, we always get the frame we asked for.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use libmpv2::{Mpv, events::Event};
use slint::{Rgba8Pixel, SharedPixelBuffer};

#[cfg(test)]
static LAST_SHOWN: Mutex<i64> = Mutex::new(-1);

#[derive(Default)]
struct Wanted {
    path: Option<String>,
    frames: VecDeque<(f64, i64)>, // (target time in s, frame index), in priority order
}

#[derive(Clone)]
pub struct Preview(Arc<(Mutex<Wanted>, Condvar)>);

impl Preview {
    /// `on_image(number, image)` is called from the preview thread.
    pub fn spawn(on_image: impl Fn(i64, SharedPixelBuffer<Rgba8Pixel>) + Send + 'static) -> Self {
        let shared = Arc::new((Mutex::new(Wanted::default()), Condvar::new()));
        let state = shared.clone();
        std::thread::spawn(move || {
            let Ok(mpv) = Mpv::with_initializer(|i| {
                i.set_option("vo", "libmpv")?;
                i.set_option("audio", "no")?;
                i.set_option("sid", "no")?;
                i.set_option("pause", "yes")?;
                i.set_option("keep-open", "yes")?;
                i.set_option("idle", "yes")?;
                i.set_option("hr-seek", "yes")?;
                i.set_option("hwdec", "auto-copy")?;
                i.set_option("vd-lavc-threads", 8i64)?; // leave most cores to the main engine
                i.set_option("vf", "scale=w=320:h=-2")?;
                Ok(())
            }) else {
                return;
            };
            let Some(render) = SwRender::new(&mpv) else {
                return;
            };
            let (lock, cvar) = &*state;
            loop {
                let (path, frame) = {
                    let mut w = lock.lock().unwrap();
                    while w.path.is_none() && w.frames.is_empty() {
                        w = cvar.wait(w).unwrap();
                    }
                    // one frame at a time: a new request replaces the rest of the list
                    (w.path.take(), w.frames.pop_front())
                };
                if let Some(p) = path {
                    let _ = mpv.command("loadfile", &[&p]);
                    wait_for(&mpv, |e| matches!(e, Event::FileLoaded));
                    render.size.set(None);
                }
                if let Some((t, n)) = frame
                    && {
                        render.forget_updates();
                        mpv.command("seek", &[&t.to_string(), "absolute+exact"]).is_ok()
                    }
                    && wait_for(&mpv, |e| matches!(e, Event::PlaybackRestart))
                    && let Some(img) = render.next_frame(&mpv, n)
                {
                    #[cfg(test)]
                    {
                        let pos = mpv.get_property::<f64>("time-pos").unwrap_or(-1.0);
                        let fps = mpv.get_property::<f64>("container-fps").unwrap_or(1.0);
                        *LAST_SHOWN.lock().unwrap() = (pos * fps).round() as i64;
                    }
                    on_image(n, img);
                }
            }
        });
        Preview(shared)
    }

    pub fn open(&self, path: &str) {
        let mut w = self.0.0.lock().unwrap();
        w.path = Some(path.to_string());
        w.frames.clear();
        self.0.1.notify_one();
    }

    /// Frames to decode, most important first; replaces the previous request.
    pub fn request(&self, frames: impl IntoIterator<Item = (f64, i64)>) {
        self.0.0.lock().unwrap().frames = frames.into_iter().collect();
        self.0.1.notify_one();
    }
}

fn wait_for(mpv: &Mpv, want: impl Fn(&Event) -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if let Some(Ok(e)) = mpv.wait_event(0.5)
            && want(&e)
        {
            return true;
        }
    }
    false
}

/// libmpv software rendering into an in-memory image.
struct SwRender {
    ctx: *mut libmpv2_sys::mpv_render_context,
    last: std::cell::RefCell<Option<(i64, Vec<u32>)>>, // last drawn frame (index, pixels)
    // Thumbnail size, read once per file: reading a property during drawing waits for mpv's core,
    // which itself waits for the drawing (up to its guard timeout, 200 ms or more)
    size: std::cell::Cell<Option<(usize, usize)>>,
}

impl SwRender {
    fn new(mpv: &Mpv) -> Option<Self> {
        use libmpv2_sys::*;
        let mut params = [
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                data: MPV_RENDER_API_TYPE_SW.as_ptr() as *mut _,
            },
            mpv_render_param { type_: mpv_render_param_type_MPV_RENDER_PARAM_INVALID, data: std::ptr::null_mut() },
        ];
        let mut ctx = std::ptr::null_mut();
        let ok = unsafe { mpv_render_context_create(&mut ctx, mpv.ctx.as_ptr(), params.as_mut_ptr()) } >= 0;
        ok.then_some(SwRender { ctx, last: Default::default(), size: Default::default() })
    }

    /// Before a seek: discards the "new frame" signals already received.
    fn forget_updates(&self) {
        unsafe { libmpv2_sys::mpv_render_context_update(self.ctx) };
    }

    fn wait_update(&self, max: Duration) -> bool {
        use libmpv2_sys::*;
        let start = Instant::now();
        loop {
            if unsafe { mpv_render_context_update(self.ctx) } & mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME as u64 != 0 {
                return true;
            }
            if start.elapsed() > max {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Once the seek is done: waits for mpv to signal the frame, draws it (rgb0) and converts it to RGBA.
    /// A stale signal may still point to the previous frame: if we redraw exactly the same frame while aiming at another,
    /// we wait for the next signal (300 ms at most, for genuinely identical frames).
    fn next_frame(&self, mpv: &Mpv, n: i64) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        if !self.wait_update(Duration::from_secs(2)) {
            return None;
        }
        let (w, h, stride, mut mem) = self.draw(mpv)?;
        let stale = self.last.borrow().as_ref().is_some_and(|(k, px)| *k != n && *px == mem);
        if stale && self.wait_update(Duration::from_millis(300)) {
            mem = self.draw(mpv)?.3;
        }
        let bytes: &[u8] = unsafe { std::slice::from_raw_parts(mem.as_ptr() as *const u8, mem.len() * 4) };
        let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w as u32, h as u32);
        for (y, row) in buf.make_mut_slice().chunks_mut(w).enumerate() {
            for (x, px) in row.iter_mut().enumerate() {
                let s = &bytes[y * stride + x * 4..];
                *px = Rgba8Pixel { r: s[0], g: s[1], b: s[2], a: 255 };
            }
        }
        *self.last.borrow_mut() = Some((n, std::mem::take(&mut mem)));
        Some(buf)
    }

    /// Draws mpv's current frame: (width, height, bytes per line, rgb0 pixels).
    fn draw(&self, mpv: &Mpv) -> Option<(usize, usize, usize, Vec<u32>)> {
        use libmpv2_sys::*;
        let (w, h) = match self.size.get() {
            Some(s) => s,
            None => {
                let s = (
                    mpv.get_property::<i64>("dwidth").ok()?.max(1) as usize,
                    mpv.get_property::<i64>("dheight").ok()?.max(1) as usize,
                );
                self.size.set(Some(s));
                s
            }
        };
        let stride = (w * 4).div_ceil(64) * 64; // multiple of 64 bytes, as recommended by libmpv
        let mut mem = vec![0u32; stride / 4 * h]; // u32: 4-byte aligned, required for rgb0
        let mut size = [w as i32, h as i32];
        let mut stride_v = stride;
        let mut no_wait: i32 = 0; // don't wait for the frame's display time: this is a thumbnail
        let mut params = [
            mpv_render_param {
                type_: mpv_render_param_type_MPV_RENDER_PARAM_BLOCK_FOR_TARGET_TIME,
                data: &mut no_wait as *mut i32 as *mut _,
            },
            mpv_render_param { type_: mpv_render_param_type_MPV_RENDER_PARAM_SW_SIZE, data: size.as_mut_ptr() as *mut _ },
            mpv_render_param { type_: mpv_render_param_type_MPV_RENDER_PARAM_SW_FORMAT, data: c"rgb0".as_ptr() as *mut _ },
            mpv_render_param { type_: mpv_render_param_type_MPV_RENDER_PARAM_SW_STRIDE, data: &mut stride_v as *mut usize as *mut _ },
            mpv_render_param { type_: mpv_render_param_type_MPV_RENDER_PARAM_SW_POINTER, data: mem.as_mut_ptr() as *mut _ },
            mpv_render_param { type_: mpv_render_param_type_MPV_RENDER_PARAM_INVALID, data: std::ptr::null_mut() },
        ];
        if unsafe { mpv_render_context_render(self.ctx, params.as_mut_ptr()) } < 0 {
            return None;
        }
        Some((w, h, stride, mem))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Needs libmpv (target/release in PATH) and a video: STARPLAY_TEST_VIDEO=<path>, STARPLAY_TEST_FPS.
    // Writes the preview as raw RGBA to STARPLAY_TEST_OUT if set (to compare with ffmpeg).
    #[test]
    #[ignore]
    fn preview_returns_the_requested_frame() {
        let path = std::env::var("STARPLAY_TEST_VIDEO").unwrap();
        let fps: f64 = std::env::var("STARPLAY_TEST_FPS").unwrap().parse().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let p = Preview::spawn(move |n, img| {
            let _ = tx.send((n, img));
        });
        p.open(&path);
        p.request([(123.0 / fps, 123)]);
        let (n, img) = rx.recv_timeout(Duration::from_secs(30)).unwrap();
        assert_eq!(n, 123);
        assert_eq!(img.width(), 320);
        // Distant frames (hence very different): each thumbnail is written to be compared with ffmpeg
        if let Ok(dir) = std::env::var("STARPLAY_TEST_DUMP") {
            for want in [10i64, 300, 150, 0, 220, 10, 123, 300, 9, 11, 150] {
                let t0 = Instant::now();
                p.request([(want as f64 / fps, want)]);
                let (n, img) = rx.recv_timeout(Duration::from_secs(30)).unwrap();
                assert_eq!(n, want);
                eprintln!("frame {want}: {} ms", t0.elapsed().as_millis());
                let k = std::fs::read_dir(&dir).unwrap().count();
                std::fs::write(format!("{dir}/{k:02}_p{n}_{}x{}.rgba", img.width(), img.height()), img.as_bytes()).unwrap();
            }
        }
        for want in [0i64, 1, 61, 200, 251] {
            p.request([(want as f64 / fps, want)]);
            let (n, _) = rx.recv_timeout(Duration::from_secs(30)).unwrap();
            assert_eq!(n, want);
            assert_eq!(*LAST_SHOWN.lock().unwrap(), want, "frame actually shown by the preview engine");
        }
        if let Ok(out) = std::env::var("STARPLAY_TEST_OUT") {
            std::fs::write(out, img.as_bytes()).unwrap();
        }
    }
}
