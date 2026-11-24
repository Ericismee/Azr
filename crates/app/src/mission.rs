use crate::popup::Popup;
use crate::prefs;
use azr_core::mission::{grid, Rect};
use azr_platform::icons::Bitmap;
use azr_platform::log;
use azr_platform::text::{measure, render, Style};
use azr_platform::wins::{enumerate, focus, snapshot, thumbnail, tracker};
use azr_ui::gfx::Gfx;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::Color;
use windows::UI::Composition::{
    CompositionDrawingSurface, CompositionStretch, Compositor, SpriteVisual,
};
use windows_numerics::{Vector2, Vector3};

const T_ANIM: usize = 1;
const WM_ANIMATION_FRAME: u32 = WM_APP + 1;
const ANIMATION_TIME: Duration = Duration::from_millis(190);
const READY: u8 = 0;
const POSTED: u8 = 1;
const STOPPED: u8 = 2;
static NEXT_FRAME_ID: AtomicUsize = AtomicUsize::new(1);

struct FramePump {
    id: usize,
    state: Arc<AtomicU8>,
}

impl FramePump {
    fn start(hwnd: HWND, start: Instant, refresh_hz: u32) -> std::io::Result<Self> {
        let id = NEXT_FRAME_ID.fetch_add(1, Ordering::Relaxed);
        let state = Arc::new(AtomicU8::new(READY));
        let worker_state = state.clone();
        let target = hwnd.0 as isize;
        let period = Duration::from_secs_f64(1.0 / f64::from(refresh_hz.clamp(30, 360)));
        std::thread::Builder::new()
            .name("azr-overview-frames".into())
            .spawn(move || {
                let end = start + ANIMATION_TIME;
                let mut next = start + period;
                loop {
                    std::thread::sleep(next.saturating_duration_since(Instant::now()));
                    if worker_state.load(Ordering::Acquire) == STOPPED {
                        break;
                    }
                    if worker_state
                        .compare_exchange(READY, POSTED, Ordering::AcqRel, Ordering::Relaxed)
                        .is_ok()
                    {
                        let sent = unsafe {
                            PostMessageW(
                                Some(HWND(target as *mut _)),
                                WM_ANIMATION_FRAME,
                                WPARAM(id),
                                LPARAM(0),
                            )
                        };
                        if sent.is_err() {
                            break;
                        }
                    }
                    let now = Instant::now();
                    if now >= end {
                        break;
                    }
                    // Advance the deadline without replaying frames missed by a busy UI thread.
                    while next <= now {
                        next += period;
                    }
                    next = next.min(end);
                }
            })?;
        Ok(Self { id, state })
    }
}

impl Drop for FramePump {
    fn drop(&mut self) {
        self.state.store(STOPPED, Ordering::Release);
    }
}

struct Tile {
    hwnd: isize,
    handle: Option<isize>,
    from: Rect,
    to: Rect,
    title: String,
    last_rect: Option<(i32, i32, i32, i32)>,
}

impl Drop for Tile {
    fn drop(&mut self) {
        if let Some(handle) = self.handle {
            thumbnail::unregister(handle);
        }
    }
}

struct Mc {
    popup: Popup,
    gfx: Gfx,
    comp: Compositor,
    dpi: f32,
    tiles: Vec<Tile>,
    t0: Instant,
    refresh_hz: u32,
    intervals: Option<Vec<f32>>,
    last_tick: Option<Instant>,
    frames: Option<FramePump>,
    done: bool,
    hover: Option<usize>,
    switch_selection: Option<usize>,
    highlight: SpriteVisual,
    desks: Vec<(Rect, usize)>,
    plus: Option<Rect>,
    cur_desk: usize,
    text_cache: HashMap<String, (CompositionDrawingSurface, i32, i32)>,
}

struct Ctl {
    comp: Compositor,
}

thread_local! {
    static MC: RefCell<Option<Box<Mc>>> = const { RefCell::new(None) };
    static CTL: RefCell<Option<Ctl>> = const { RefCell::new(None) };
}

fn with_mc<R>(f: impl FnOnce(&mut Mc) -> R) -> Option<R> {
    MC.with(|c| {
        c.try_borrow_mut()
            .ok()
            .and_then(|mut b| b.as_mut().map(|b| f(b)))
    })
}

fn recolor(b: &Bitmap, rgb: [u8; 3]) -> Vec<u8> {
    let mut out = b.bgra.clone();
    for px in out.chunks_exact_mut(4) {
        let a = px[3] as u32;
        px[0] = (rgb[2] as u32 * a / 255) as u8;
        px[1] = (rgb[1] as u32 * a / 255) as u8;
        px[2] = (rgb[0] as u32 * a / 255) as u8;
    }
    out
}

pub fn init(comp: &Compositor) -> windows_core::Result<()> {
    unsafe {
        let hi: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = w!("AzrMissionCtl");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(ctl_proc),
            hInstance: hi,
            lpszClassName: class,
            ..Default::default()
        });
        let _hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("Azr-mc-ctl"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hi),
            None,
        )?;
        CTL.with(|c| *c.borrow_mut() = Some(Ctl { comp: comp.clone() }));
    }
    Ok(())
}

pub fn toggle() {
    if MC.with(|c| c.borrow().is_some()) {
        close();
    } else {
        show();
    }
}

pub fn show() {
    if crate::fullscreen::on_selected() {
        return;
    }
    if MC.with(|c| c.borrow().is_some()) {
        return;
    }
    let comp = CTL.with(|c| c.borrow().as_ref().map(|c| c.comp.clone()));
    let Some(comp) = comp else { return };
    match create(&comp) {
        Ok(Some(m)) => {
            MC.with(|c| *c.borrow_mut() = Some(m));
            with_mc(|m| {
                m.t0 = Instant::now();
                m.step();
                m.popup.show_active();
                crate::topmost::request_raise();
                match FramePump::start(m.popup.hwnd, m.t0, m.refresh_hz) {
                    Ok(frames) => m.frames = Some(frames),
                    Err(_) => unsafe {
                        if SetTimer(Some(m.popup.hwnd), T_ANIM, 16, None) == 0 {
                            m.t0 = Instant::now() - ANIMATION_TIME;
                            m.step();
                        }
                    },
                }
            });
        }
        Ok(None) => log::line("mission", "tidak ada jendela untuk ditampilkan"),
        Err(e) => log::line("mission", &format!("gagal: {e}")),
    }
}

pub fn step_switch(reverse: bool) {
    let active = focus::foreground();
    show();
    with_mc(|m| {
        let count = m.tiles.len();
        let selected = match m.switch_selection {
            Some(i) if reverse => (i + count - 1) % count,
            Some(i) => (i + 1) % count,
            None => {
                let mut other = m
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(_, tile)| tile.hwnd != active);
                let first = if reverse {
                    other.next_back()
                } else {
                    other.next()
                };
                first.map_or(0, |(i, _)| i)
            }
        };
        m.switch_selection = Some(selected);
        m.set_hover(Some(selected));
    });
}

pub fn accept_alt_tab() {
    if with_mc(|m| m.switch_selection.is_some()).unwrap_or(false) {
        commit_switch();
    }
}

pub fn commit_switch() {
    let target = with_mc(|m| m.switch_selection.or(m.hover).map(|i| m.tiles[i].hwnd)).flatten();
    close();
    if let Some(hwnd) = target {
        let activated = focus::force_foreground(hwnd);
        if std::env::var_os("AZR_UI_TIMINGS").is_some() {
            log::line(
                "overview",
                &format!("switch target={hwnd} activated={activated}"),
            );
        }
    }
}

pub fn close_for_fullscreen() {
    if with_mc(|m| crate::fullscreen::on_window(m.popup.hwnd)).unwrap_or(false) {
        close();
    }
}

pub fn close() {
    let m = MC.with(|c| c.borrow_mut().take());
    if let Some(m) = m {
        let h = m.popup.hwnd;
        drop(m);
        unsafe {
            let _ = DestroyWindow(h);
        }
        azr_platform::sys::trim_later(5000);
    }
}

fn create(comp: &Compositor) -> windows_core::Result<Option<Box<Mc>>> {
    let display = crate::display::selected();
    let screen = display.size();
    let (ox, oy) = display.origin();
    let mut look = prefs::look(prefs::Role::Dark);
    look.blur_sigma = look.blur_sigma.clamp(12.0, 20.0);
    look.tint = [72, 16, 20, 30];
    let base = azr_ui::glass::GlassParams::dark((10.0, 10.0), 0.0);
    let mut popup = Popup::new_with(
        comp,
        w!("AzrMission"),
        Some(mc_proc),
        WINDOW_EX_STYLE(0),
        prefs::apply(base, &look),
        true,
    )?;
    popup.set_rect(ox, oy, screen.0, screen.1)?;
    let dpi = unsafe { GetDpiForWindow(popup.hwnd) }.max(96) as f32 / 96.0;
    popup.panel.set_radius(0.0)?;
    let dst = popup.hwnd.0 as isize;

    let mut wins: Vec<_> = enumerate::list(std::process::id()).into_iter().collect();
    wins.sort_by_key(|w| std::cmp::Reverse(tracker::recent(w.hwnd)));
    let mut tiles = Vec::new();
    let mut sizes = Vec::new();
    for w in &wins {
        let handle = if focus::is_minimized(w.hwnd) {
            None
        } else {
            thumbnail::register(dst, w.hwnd)
        };
        let size = handle.and_then(thumbnail::source_size);
        let (sw, sh) = size.unwrap_or((480, 300));
        let (fx, fy, fw, fh) = if size.is_some() {
            snapshot::frame_rect(w.hwnd).unwrap_or((ox, oy, sw, sh))
        } else {
            (ox, oy, sw, sh)
        };
        sizes.push((sw as f32, sh as f32));
        let title = if w.name.is_empty() {
            focus::title(w.hwnd)
        } else {
            w.name.clone()
        };
        tiles.push(Tile {
            hwnd: w.hwnd,
            handle: handle.filter(|handle| {
                if size.is_none() {
                    thumbnail::unregister(*handle);
                }
                size.is_some()
            }),
            from: Rect {
                x: (fx - ox) as f32,
                y: (fy - oy) as f32,
                w: fw as f32,
                h: fh as f32,
            },
            to: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            title,
            last_rect: None,
        });
    }
    if tiles.is_empty() {
        unsafe {
            let _ = DestroyWindow(popup.hwnd);
        }
        return Ok(None);
    }
    let area = Rect {
        x: 60.0 * dpi,
        y: 124.0 * dpi,
        w: screen.0 as f32 - 120.0 * dpi,
        h: screen.1 as f32 - 172.0 * dpi,
    };
    let g = grid(&sizes, area, 24.0 * dpi, 34.0 * dpi);
    for (t, r) in tiles.iter_mut().zip(g) {
        t.to = r;
    }
    let highlight = comp.CreateSpriteVisual()?;
    highlight.SetOpacity(0.0)?;
    popup.content.Children()?.InsertAtTop(&highlight)?;
    let gfx = Gfx::shared(comp)?;
    Ok(Some(Box::new(Mc {
        popup,
        gfx,
        comp: comp.clone(),
        dpi,
        tiles,
        t0: Instant::now(),
        refresh_hz: display.refresh_hz(),
        intervals: std::env::var_os("AZR_FRAME_LOG").map(|_| Vec::with_capacity(24)),
        last_tick: None,
        frames: None,
        done: false,
        hover: None,
        switch_selection: None,
        highlight,
        desks: vec![],
        plus: None,
        cur_desk: 0,
        text_cache: HashMap::new(),
    })))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Mc {
    fn step(&mut self) {
        let t = (self.t0.elapsed().as_secs_f32() / ANIMATION_TIME.as_secs_f32()).clamp(0.0, 1.0);
        let e = 1.0 - (1.0 - t).powi(3);
        if let Some(intervals) = self.intervals.as_mut() {
            let now = Instant::now();
            if let Some(last) = self.last_tick.replace(now) {
                intervals.push((now - last).as_secs_f32() * 1000.0);
            }
        }
        for tile in &mut self.tiles {
            let (f, to) = (tile.from, tile.to);
            let r = (
                lerp(f.x, to.x, e).round() as i32,
                lerp(f.y, to.y, e).round() as i32,
                (lerp(f.x, to.x, e) + lerp(f.w, to.w, e)).round() as i32,
                (lerp(f.y, to.y, e) + lerp(f.h, to.h, e)).round() as i32,
            );
            if tile.last_rect != Some(r) {
                if let Some(handle) = tile.handle {
                    thumbnail::place(handle, r, 255);
                }
                tile.last_rect = Some(r);
            }
        }
        if t >= 1.0 && !self.done {
            self.done = true;
            self.frames = None;
            unsafe {
                let _ = KillTimer(Some(self.popup.hwnd), T_ANIM);
            }
            if let Some(mut samples) = self.intervals.take().filter(|values| !values.is_empty()) {
                samples.sort_by(f32::total_cmp);
                let median = samples[samples.len() / 2];
                let max = samples[samples.len() - 1];
                log::line(
                    "overview",
                    &format!(
                        "refresh_hz={} samples={} median_ms={median:.1} max_ms={max:.1}",
                        self.refresh_hz,
                        samples.len()
                    ),
                );
            }
            self.draw_desktops();
            self.draw_window_labels();
            let active = self.hover.take();
            self.set_hover(active);
        }
    }

    fn draw_window_labels(&mut self) {
        let dpi = self.dpi;
        for i in 0..self.tiles.len() {
            let tile = &self.tiles[i];
            let r = tile.to;
            let title = tile.title.clone();
            let missing = tile.handle.is_none();
            self.card(
                Rect {
                    x: r.x - 5.0 * dpi,
                    y: r.y - 5.0 * dpi,
                    w: r.w + 10.0 * dpi,
                    h: r.h + 34.0 * dpi,
                },
                12.0 * dpi,
                Color {
                    A: 52,
                    R: 145,
                    G: 169,
                    B: 204,
                },
            );
            if missing {
                let _ = self.put_text(
                    prefs::tr("Preview unavailable"),
                    (12.0 * dpi) as i32,
                    500,
                    r.x,
                    r.y + r.h / 2.0,
                    r.w,
                    0.75,
                );
            }
            let _ = self.put_text(
                &title,
                (12.0 * dpi) as i32,
                600,
                r.x,
                r.y + r.h + 7.0 * dpi,
                r.w,
                0.95,
            );
        }
    }

    fn put_text(
        &mut self,
        text: &str,
        px: i32,
        weight: i32,
        x: f32,
        y: f32,
        maxw: f32,
        op: f32,
    ) -> Option<SpriteVisual> {
        let key = format!("{px}|{weight}|{text}|{}", maxw as i32);
        let hit = self.text_cache.get(&key).cloned();
        let (surf, w, h) = match hit {
            Some(v) => v,
            None => {
                let style = Style { px, weight };
                let (tw, th) = measure(text, &style);
                let tw = (tw + 2).min(maxw.max(40.0) as i32);
                let bmp = render(text, &style, tw, th)?;
                let sf = self
                    .gfx
                    .surface_from_bgra(bmp.w, bmp.h, &recolor(&bmp, [255, 255, 255]))
                    .ok()?;
                let v = (sf, tw, th);
                self.text_cache.insert(key, v.clone());
                v
            }
        };
        let go = || -> windows_core::Result<SpriteVisual> {
            let b = self.comp.CreateSurfaceBrushWithSurface(&surf)?;
            b.SetStretch(CompositionStretch::None)?;
            let v = self.comp.CreateSpriteVisual()?;
            v.SetBrush(&b)?;
            v.SetSize(Vector2 {
                X: w as f32,
                Y: h as f32,
            })?;
            v.SetOffset(Vector3 {
                X: x + (maxw - w as f32) / 2.0,
                Y: y,
                Z: 0.0,
            })?;
            v.SetOpacity(op)?;
            self.popup.content.Children()?.InsertAtTop(&v)?;
            Ok(v)
        };
        go().ok()
    }

    fn card(&self, r: Rect, radius: f32, c: Color) {
        let go = || -> windows_core::Result<()> {
            let v = self.comp.CreateSpriteVisual()?;
            v.SetSize(Vector2 { X: r.w, Y: r.h })?;
            v.SetOffset(Vector3 {
                X: r.x,
                Y: r.y,
                Z: 0.0,
            })?;
            v.SetBrush(&self.comp.CreateColorBrushWithColor(c)?)?;
            let g = self.comp.CreateRoundedRectangleGeometry()?;
            g.SetSize(Vector2 { X: r.w, Y: r.h })?;
            g.SetCornerRadius(Vector2 {
                X: radius,
                Y: radius,
            })?;
            let clip = self.comp.CreateGeometricClip()?;
            clip.SetGeometry(&g)?;
            v.SetClip(&clip)?;
            self.popup.content.Children()?.InsertAtTop(&v)
        };
        let _ = go();
    }

    fn draw_desktops(&mut self) {
        let d = self.dpi;
        let (names, cur) = azr_platform::vdesk::list();
        self.cur_desk = cur;
        let (cw, ch, gap) = (144.0 * d, 64.0 * d, 12.0 * d);
        let n = names.len() as f32;
        let plus_w = 48.0 * d;
        let total = n * cw + (n - 1.0) * gap + gap + plus_w;
        let sw = self.popup.rect.2 as f32;
        let x0 = (sw - total) / 2.0;
        let y = 37.0 * d;
        self.desks.clear();
        for (i, name) in names.iter().enumerate() {
            let r = Rect {
                x: x0 + i as f32 * (cw + gap),
                y,
                w: cw,
                h: ch,
            };
            if i == cur {
                self.card(
                    Rect {
                        x: r.x - 3.0 * d,
                        y: r.y - 3.0 * d,
                        w: r.w + 6.0 * d,
                        h: r.h + 6.0 * d,
                    },
                    14.0 * d,
                    crate::prefs::accent_c(175),
                );
            }
            self.card(
                r,
                12.0 * d,
                Color {
                    A: 110,
                    R: 255,
                    G: 255,
                    B: 255,
                },
            );
            let _ = self.put_text(
                name,
                (13.0 * d) as i32,
                600,
                r.x,
                r.y + r.h / 2.0 - 8.0 * d,
                r.w,
                1.0,
            );
            self.desks.push((r, i));
        }
        let pr = Rect {
            x: x0 + n * (cw + gap),
            y,
            w: plus_w,
            h: ch,
        };
        self.card(
            pr,
            12.0 * d,
            Color {
                A: 70,
                R: 255,
                G: 255,
                B: 255,
            },
        );
        let _ = self.put_text(
            "+",
            (28.0 * d) as i32,
            300,
            pr.x,
            pr.y + pr.h / 2.0 - 18.0 * d,
            pr.w,
            1.0,
        );
        self.plus = Some(pr);
    }

    fn tile_at(&self, x: f32, y: f32) -> Option<usize> {
        self.tiles.iter().position(|tile| {
            if self.done {
                let r = tile.to;
                x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h + 25.0 * self.dpi
            } else {
                tile.last_rect.is_some_and(|(l, t, r, b)| {
                    x >= l as f32 && x < r as f32 && y >= t as f32 && y < b as f32
                })
            }
        })
    }

    fn set_hover(&mut self, i: Option<usize>) {
        if i == self.hover {
            return;
        }
        self.hover = i;
        match i.map(|i| self.tiles[i].to) {
            Some(r) if self.done => {
                let pad = 7.0 * self.dpi;
                let go = || -> windows_core::Result<()> {
                    self.highlight.SetSize(Vector2 {
                        X: r.w + 2.0 * pad,
                        Y: r.h + 2.0 * pad + 22.0 * self.dpi,
                    })?;
                    self.highlight.SetOffset(Vector3 {
                        X: r.x - pad,
                        Y: r.y - pad,
                        Z: 0.0,
                    })?;
                    self.highlight.SetBrush(
                        &self
                            .comp
                            .CreateColorBrushWithColor(crate::prefs::accent_c(108))?,
                    )?;
                    let g = self.comp.CreateRoundedRectangleGeometry()?;
                    g.SetSize(Vector2 {
                        X: r.w + 2.0 * pad,
                        Y: r.h + 2.0 * pad + 22.0 * self.dpi,
                    })?;
                    g.SetCornerRadius(Vector2 {
                        X: 10.0 * self.dpi,
                        Y: 10.0 * self.dpi,
                    })?;
                    let clip = self.comp.CreateGeometricClip()?;
                    clip.SetGeometry(&g)?;
                    self.highlight.SetClip(&clip)?;
                    self.highlight.SetOpacity(1.0)
                };
                let _ = go();
            }
            _ => {
                let _ = self.highlight.SetOpacity(0.0);
            }
        }
    }
}

unsafe extern "system" fn ctl_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            toggle();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn mc_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            close();
            LRESULT(0)
        }
        WM_ANIMATION_FRAME => {
            with_mc(|m| {
                if let Some(frames) = m.frames.as_ref().filter(|frames| frames.id == wp.0) {
                    frames.state.store(READY, Ordering::Release);
                    m.step();
                }
            });
            LRESULT(0)
        }
        WM_TIMER if wp.0 == T_ANIM => {
            with_mc(|m| m.step());
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = ((lp.0 as i16) as f32, ((lp.0 >> 16) as i16) as f32);
            with_mc(|m| {
                if m.switch_selection.is_none() {
                    let i = m.tile_at(x, y);
                    m.set_hover(i);
                }
            });
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let key = wp.0 as u16;
            if key == VK_ESCAPE.0 {
                close();
            } else if key == VK_RETURN.0 {
                commit_switch();
            } else if [VK_LEFT.0, VK_RIGHT.0, VK_UP.0, VK_DOWN.0].contains(&key) {
                with_mc(|m| {
                    let Some(index) = m.hover else {
                        m.set_hover(Some(0));
                        return;
                    };
                    let origin = m.tiles[index].to;
                    let mut candidates = m.tiles.iter().enumerate().filter_map(|(i, tile)| {
                        let rect = tile.to;
                        let dx = rect.x + rect.w / 2.0 - (origin.x + origin.w / 2.0);
                        let dy = rect.y + rect.h / 2.0 - (origin.y + origin.h / 2.0);
                        let (forward, lateral) = if key == VK_LEFT.0 {
                            (-dx, dy)
                        } else if key == VK_RIGHT.0 {
                            (dx, dy)
                        } else if key == VK_UP.0 {
                            (-dy, dx)
                        } else {
                            (dy, dx)
                        };
                        (forward > 1.0).then_some((i, forward + 2.0 * lateral.abs()))
                    });
                    if let Some((i, _)) = candidates.by_ref().min_by(|a, b| a.1.total_cmp(&b.1)) {
                        if m.switch_selection.is_some() {
                            m.switch_selection = Some(i);
                        }
                        m.set_hover(Some(i));
                    }
                });
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = ((lp.0 as i16) as f32, ((lp.0 >> 16) as i16) as f32);
            let in_r = |r: &Rect| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h;
            let desk = with_mc(|m| {
                if let Some(p) = m.plus.filter(|p| in_r(p)) {
                    let _ = p;
                    return Some(Err(()));
                }
                m.desks
                    .iter()
                    .find(|(r, _)| in_r(r))
                    .map(|(_, i)| Ok((*i as i32) - (m.cur_desk as i32)))
            })
            .flatten();
            if let Some(act) = desk {
                close();
                match act {
                    Err(()) => azr_platform::keys::chord(&[
                        azr_platform::keys::WIN,
                        azr_platform::keys::CTRL,
                        0x44,
                    ]),
                    Ok(delta) => {
                        for _ in 0..delta.abs() {
                            azr_platform::keys::chord(&[
                                azr_platform::keys::WIN,
                                azr_platform::keys::CTRL,
                                if delta > 0 { 0x27 } else { 0x25 },
                            ]);
                        }
                    }
                }
                return LRESULT(0);
            }
            let target = with_mc(|m| m.tile_at(x, y).map(|i| m.tiles[i].hwnd)).flatten();
            close();
            if let Some(h) = target {
                focus::force_foreground(h);
            }
            LRESULT(0)
        }
        WM_ACTIVATE => {
            if (wp.0 & 0xFFFF) == 0 {
                let _ = PostMessageW(Some(hwnd), WM_APP + 90, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        m if m == WM_APP + 90 => {
            close();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
