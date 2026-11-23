use crate::popup::Popup;
use crate::prefs::{self, Role, WM_PREFS};
use azr_core::hud::slider;
use azr_core::i18n::Lang;
use azr_core::settings::{Behavior, Material, Theme};
use azr_platform::icons::Bitmap;
use azr_platform::text::{measure, render, render_face, Style, ICON_FACE};
use azr_platform::{log, sys};
use azr_ui::gfx::Gfx;
use std::cell::RefCell;
use std::collections::HashMap;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
    VK_CONTROL,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::Color;
use windows::UI::Composition::{
    CompositionDrawingSurface, CompositionStretch, Compositor, ContainerVisual,
};
const WM_MOUSELEAVE: u32 = 0x02A3;
use windows_core::Interface;
use windows_numerics::{Vector2, Vector3};

const WM_CLOSE_SELF: u32 = WM_APP + 50;
const W: f32 = 780.0;
const H: f32 = 560.0;
const SIDEBAR_W: f32 = 214.0;
const CONTENT_X: f32 = SIDEBAR_W + 24.0;
const CONTENT_W: f32 = W - SIDEBAR_W - 48.0;
const VIEW_TOP: f32 = 56.0;
const SIDE_INSET: f32 = 8.0;
const SIDE_RADIUS: f32 = 18.0;
const SIDE_TOP: f32 = 88.0;
const SIDE_BOTTOM: f32 = H - 48.0;

const PAGES: [(&str, &str, [u8; 3], u8); 10] = [
    ("General", "\u{E713}", [142, 142, 147], 0),
    ("Dock", "\u{E7C4}", [10, 132, 255], 1),
    ("Dynamic Island", "\u{E7F4}", [58, 58, 60], 2),
    ("Menu bar", "\u{E700}", [90, 200, 250], 3),
    ("Appearance", "\u{E790}", [88, 86, 214], 4),
    ("Window manager", "\u{E8A0}", [48, 209, 88], 5),
    ("Screen Corners", "\u{E73F}", [175, 82, 222], 6),
    ("Quick Search", "\u{E721}", [255, 149, 10], 7),
    ("Widgets", "\u{E737}", [0, 122, 255], 8),
    ("About & Support", "\u{E946}", [68, 110, 183], 9),
];

fn fold_search(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|ch| {
            Some(match ch {
                'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ'
                | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' => 'a',
                'đ' => 'd',
                'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => {
                    'e'
                }
                'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
                'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ'
                | 'ớ' | 'ở' | 'ỡ' | 'ợ' => 'o',
                'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => {
                    'u'
                }
                'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
                '\u{0300}'..='\u{036f}' => return None,
                _ => ch,
            })
        })
        .collect()
}

fn search_matches(label: &str, query: &str) -> bool {
    let label = fold_search(label);
    query.split_whitespace().all(|word| label.contains(word))
}

fn side_glass(mut g: azr_ui::glass::GlassParams) -> azr_ui::glass::GlassParams {
    g.blur_sigma = g.blur_sigma.max(16.0);
    g
}

fn round_sdf(px: f32, py: f32, r: (f32, f32, f32, f32), rad: f32) -> f32 {
    let (cx, cy) = (r.0 + r.2 / 2.0, r.1 + r.3 / 2.0);
    let rad = rad.min(r.2 / 2.0).min(r.3 / 2.0);
    let qx = (px - cx).abs() - (r.2 / 2.0 - rad);
    let qy = (py - cy).abs() - (r.3 / 2.0 - rad);
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - rad
}

fn frame_pixels(
    w: i32,
    h: i32,
    radius: f32,
    hole: (f32, f32, f32, f32),
    hole_radius: f32,
    dark: bool,
) -> Vec<u8> {
    let base: [f32; 3] = if dark {
        [16.0, 16.0, 18.0]
    } else {
        [255.0, 255.0, 255.0]
    };
    let (edge, edge_a): ([f32; 3], f32) = if dark {
        ([255.0, 255.0, 255.0], 0.16)
    } else {
        ([0.0, 0.0, 0.0], 0.12)
    };
    let outer = (0.0, 0.0, w as f32, h as f32);
    let blur = hole_radius * 1.1;
    let shade_dy = hole_radius * 0.12;
    let (tone, peak, inner): ([f32; 3], f32, f32) = if dark {
        ([255.0, 255.0, 255.0], 0.0, 0.07)
    } else {
        ([0.0, 0.0, 0.0], 0.10, 0.05)
    };
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = round_sdf(px, py, outer, radius);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let a = cov;
            let sd = round_sdf(px, py - shade_dy, hole, hole_radius);
            let shade = if sd <= 0.0 {
                inner
            } else {
                peak * (1.0 - (sd / blur).clamp(0.0, 1.0)).powi(3)
            };
            let ring = (cov - (0.5 - (d + 1.0)).clamp(0.0, 1.0)).clamp(0.0, 1.0) * edge_a;
            let i = ((y * w + x) * 4) as usize;
            for c in 0..3 {
                let shaded = base[c] * (1.0 - shade) + tone[c] * shade;
                let v = shaded * (1.0 - ring) + edge[c] * ring;
                out[i + 2 - c] = (v * a).round() as u8;
            }
            out[i + 3] = (a * 255.0).round() as u8;
        }
    }
    out
}

#[derive(Clone, Copy, Debug)]
enum Action {
    EditText(u8),
    NotchIdle(u8),
    LyricsCompact,
    NotchDownloads,
    NotchSend,
    LyricsOnline,
    LyricsDir,
    Close,
    Minimize,
    ProjectRepo,
    ProjectSupport,
    MainDisplay(u8),
    Takeover,
    Dock(Behavior),
    Notch(Behavior),
    Theme(Theme),
    Lang(Lang),
    Tab(u8),
    Autostart,
    Anim,
    Refraction,
    DockOn,
    MenuBarOn,
    Clock24,
    BarStyle(u8),
    HotCycle(usize),
    StageOn,
    SwitcherOn,
    StageFlag(u8),
    StageRight(bool),
    StageIcons(u8),
    HideIcons,
    DockPlates,
    DockSurface(u8),
    IconBackground(u8),
    MacIconCorners,
    IconPack,
    DownloadsStack,
    RecentApps,
    ClipHistory,
    MacKeys,
    Accent(u8),
    IconVariant(u8),
    MinEffect(u8),
    Back,
    Forward,
    Search,
    ClearSearch,
    IconStyle(u8),
    IconTint(u8),
    NotchOn,
    HoverOpen,
    WinShadow,
    Material(bool, Material),
    Slider(u8, f32, f32),
    MenuLogoPick,
    LaunchpadKey(u8),
    LaunchpadIcon(u8),
    MenuLogoMode(u8),
    MenuLogoReset,
    UiScale(u8),
    NotchShape(bool),
    WidgetStyle(u8),
    WidgetGallery,
    BatteryPct(u8),
}

struct Region {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    a: Action,
    scroll: bool,
}

enum RowDef {
    Toggle(&'static str, bool, Action),
    Seg(&'static str, Vec<(&'static str, Action)>, usize),
    Slider(&'static str, u8, u8),
    Pick(&'static str, String, Action),
    Info(&'static str, &'static str),
    Brand,
    Corners([azr_core::settings::HotAction; 4]),
    Swatches(&'static str, usize),
    Thumbs(&'static str, Vec<(&'static str, Thumb, Action)>, usize),
    Radios(Vec<(&'static str, Action)>, usize),
    Text(&'static str, String, String, u8),
}

#[derive(Clone, Copy)]
enum Thumb {
    Appear(u8),
    Icon(azr_core::settings::IconStyle),
}

const ACCENT_NAMES: [&str; 9] = [
    "Multicolor",
    "Blue",
    "Purple",
    "Pink",
    "Red",
    "Orange",
    "Yellow",
    "Green",
    "Graphite",
];

fn row_label(r: &RowDef) -> &'static str {
    match r {
        RowDef::Brand => "",
        RowDef::Corners(_) => {
            prefs::tr("Move the pointer into a screen corner to run its assigned action.")
        }

        RowDef::Toggle(l, ..)
        | RowDef::Seg(l, ..)
        | RowDef::Slider(l, ..)
        | RowDef::Info(l, ..)
        | RowDef::Pick(l, ..)
        | RowDef::Swatches(l, ..)
        | RowDef::Thumbs(l, ..)
        | RowDef::Text(l, ..) => l,
        RowDef::Radios(..) => "",
    }
}

fn user_name() -> String {
    std::env::var("USERNAME")
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Azr".to_owned())
}

struct Win {
    popup: Popup,
    gfx: Gfx,
    comp: Compositor,
    dpi: f32,
    size: (f32, f32),
    regions: Vec<Region>,
    cache: HashMap<String, (CompositionDrawingSurface, i32, i32)>,
    dragging: Option<(u8, f32, f32)>,
    side: Vec<(u8, (f32, f32, f32, f32), ContainerVisual)>,
    side_hover: Option<u8>,
    side_scroll: f32,
    side_view: (f32, f32),
    side_bar: Option<((f32, f32, f32, f32), f32)>,
    side_drag: Option<f32>,
    press: Option<(f32, f32)>,
    tab: u8,
    scroll: f32,
    content_h: f32,
    content_body: Option<ContainerVisual>,
    history: Vec<u8>,
    hpos: usize,
    search: String,
    search_focus: bool,
    thumbs: HashMap<String, CompositionDrawingSurface>,
    edit: Option<u8>,
    edit_buf: String,
    frame: Option<windows::UI::Composition::SpriteVisual>,
    frame_key: (i32, i32, bool),
}

thread_local! {
    static WIN: RefCell<Option<Box<Win>>> = const { RefCell::new(None) };
}

pub fn open(comp: &Compositor) {
    let existing = WIN.with(|w| w.borrow().as_ref().map(|b| b.popup.hwnd));
    if let Some(h) = existing {
        unsafe {
            let _ = SetWindowPos(
                h,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            let _ = SetForegroundWindow(h);
        }
        return;
    }
    match create(comp) {
        Ok(()) => log::line("settings", "dibuka"),
        Err(e) => log::line("settings", &format!("gagal: {e}")),
    }
}

pub fn open_page(comp: &Compositor, page: u8, scroll: f32) {
    open(comp);
    with_win(|w| {
        w.tab = page.min(9);
        w.scroll = scroll.max(0.0);
        w.history = vec![w.tab];
        w.hpos = 0;
        w.render();
    });
}

fn settings_work_area(hwnd: HWND) -> windows::Win32::Foundation::RECT {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        info.rcWork
    } else {
        let (w, h) = sys::screen_size();
        windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        }
    }
}

fn fitted_dpi(hwnd: HWND) -> f32 {
    let native = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32 / 96.0;
    let area = settings_work_area(hwnd);
    (native * prefs::ui_scale())
        .min(((area.right - area.left) as f32 - 24.0) / W)
        .min(((area.bottom - area.top) as f32 - 24.0) / H)
        .max(0.5)
}

fn create(comp: &Compositor) -> windows_core::Result<()> {
    let glass = side_glass(prefs::glass((10.0, 10.0), SIDE_RADIUS, Role::Themed));
    let popup = Popup::new_with(
        comp,
        w!("AzrSettings"),
        Some(proc),
        WINDOW_EX_STYLE(0),
        glass,
        true,
    )?;
    let dpi = fitted_dpi(popup.hwnd);
    let gfx = Gfx::shared(comp)?;
    let tier = {
        let c = sys::conditions();
        if !c.transparency || c.battery_saver || c.remote_session {
            azr_ui::glass::Tier::Solid
        } else {
            azr_ui::glass::Tier::Full
        }
    };
    let _ = popup.panel.set_tier(tier);
    let _ = popup.panel.set_radius(24.0 * dpi);
    let mut win = Box::new(Win {
        popup,
        gfx,
        comp: comp.clone(),
        dpi,
        size: (0.0, 0.0),
        regions: vec![],
        cache: HashMap::new(),
        dragging: None,
        side: vec![],
        side_hover: None,
        side_scroll: 0.0,
        side_view: (0.0, 0.0),
        side_bar: None,
        side_drag: None,
        press: None,
        tab: 0,
        scroll: 0.0,
        content_h: 0.0,
        content_body: None,
        history: vec![0],
        hpos: 0,
        search: String::new(),
        search_focus: false,
        thumbs: HashMap::new(),
        edit: None,
        edit_buf: String::new(),
        frame: None,
        frame_key: (0, 0, false),
    });
    win.render();
    let (sw, sh) = sys::screen_size();
    let (w, h) = (win.size.0 as i32, win.size.1 as i32);
    let area = settings_work_area(win.popup.hwnd);
    let top = (area.top + (area.bottom - area.top - h) / 2)
        .clamp(area.top + 12, (area.bottom - h - 12).max(area.top + 12));
    let left = area.left + (area.right - area.left - w) / 2;
    log::line(
        "settings",
        &format!("layar={sw}x{sh} jendela={w}x{h} atas={top}"),
    );
    win.popup.set_rect(left, top, w, h)?;
    win.frame();
    let hwnd = win.popup.hwnd;
    prefs::listen(hwnd);
    WIN.with(|s| *s.borrow_mut() = Some(win));
    with_win(|w| w.popup.show_active());
    Ok(())
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

fn col(rgb: [u8; 3], a: u8) -> Color {
    Color {
        A: a,
        R: rgb[0],
        G: rgb[1],
        B: rgb[2],
    }
}

impl Win {
    fn surface(
        &mut self,
        s: &str,
        px: i32,
        weight: i32,
        rgb: [u8; 3],
    ) -> Option<(CompositionDrawingSurface, i32, i32)> {
        let key = format!("{px}|{weight}|{rgb:?}|{s}");
        if let Some(v) = self.cache.get(&key) {
            return Some(v.clone());
        }
        if self.cache.len() > 160 {
            self.cache.clear();
        }
        let style = Style { px, weight };
        let (tw, th) = measure(s, &style);
        let tw = tw + 2;
        let bmp = render(s, &style, tw, th)?;
        let data = recolor(&bmp, rgb);
        let surf = self.gfx.surface_from_bgra(bmp.w, bmp.h, &data).ok()?;
        let v = (surf, tw, th);
        self.cache.insert(key, v.clone());
        Some(v)
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        parent: &ContainerVisual,
        s: &str,
        x: f32,
        y: f32,
        size: f32,
        weight: i32,
        rgb: [u8; 3],
        alpha: f32,
        anchor: u8,
    ) -> f32 {
        let px = (size * self.dpi).round() as i32;
        let Some((surf, w, h)) = self.surface(s, px, weight, rgb) else {
            return 0.0;
        };
        let left = match anchor {
            1 => x - w as f32 / 2.0,
            2 => x - w as f32,
            _ => x,
        };
        let make = || -> windows_core::Result<()> {
            let b = self.comp.CreateSurfaceBrushWithSurface(&surf)?;
            b.SetStretch(CompositionStretch::None)?;
            let v = self.comp.CreateSpriteVisual()?;
            v.SetBrush(&b)?;
            v.SetSize(Vector2 {
                X: w as f32,
                Y: h as f32,
            })?;
            v.SetOffset(Vector3 {
                X: left,
                Y: y,
                Z: 0.0,
            })?;
            v.SetOpacity(alpha)?;
            parent.Children()?.InsertAtTop(&v)
        };
        let _ = make();
        w as f32
    }

    fn rect(&self, parent: &ContainerVisual, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
        let make = || -> windows_core::Result<()> {
            let v = self.comp.CreateSpriteVisual()?;
            v.SetSize(Vector2 { X: w, Y: h })?;
            v.SetOffset(Vector3 { X: x, Y: y, Z: 0.0 })?;
            v.SetBrush(&self.comp.CreateColorBrushWithColor(c)?)?;
            if r > 0.0 {
                let g = self.comp.CreateRoundedRectangleGeometry()?;
                g.SetSize(Vector2 { X: w, Y: h })?;
                g.SetCornerRadius(Vector2 { X: r, Y: r })?;
                let clip = self.comp.CreateGeometricClip()?;
                clip.SetGeometry(&g)?;
                v.SetClip(&clip)?;
            }
            parent.Children()?.InsertAtTop(&v)
        };
        let _ = make();
    }

    fn glass_knob(&self, parent: &ContainerVisual, x: f32, y: f32, w: f32, h: f32) {
        let d = self.dpi;
        self.rect(
            parent,
            x - 1.0 * d,
            y + 1.0 * d,
            w + 2.0 * d,
            h + 2.0 * d,
            h / 2.0 + d,
            Color {
                A: 46,
                R: 0,
                G: 0,
                B: 0,
            },
        );
        let make = || -> windows_core::Result<()> {
            let geo = self.comp.CreateRoundedRectangleGeometry()?;
            geo.SetOffset(Vector2 { X: 0.5, Y: 0.5 })?;
            geo.SetSize(Vector2 {
                X: (w - 1.0).max(0.0),
                Y: (h - 1.0).max(0.0),
            })?;
            geo.SetCornerRadius(Vector2 {
                X: h / 2.0,
                Y: h / 2.0,
            })?;
            let shape = self.comp.CreateSpriteShapeWithGeometry(&geo)?;
            shape.SetFillBrush(&self.comp.CreateColorBrushWithColor(Color {
                A: 92,
                R: 255,
                G: 255,
                B: 255,
            })?)?;
            shape.SetStrokeBrush(&self.comp.CreateColorBrushWithColor(Color {
                A: 190,
                R: 255,
                G: 255,
                B: 255,
            })?)?;
            shape.SetStrokeThickness(1.0)?;
            let v = self.comp.CreateShapeVisual()?;
            v.SetSize(Vector2 { X: w, Y: h })?;
            v.SetOffset(Vector3 { X: x, Y: y, Z: 0.0 })?;
            v.Shapes()?.Append(&shape)?;
            parent.Children()?.InsertAtTop(&v)
        };
        let _ = make();
    }

    fn glyph_icon(
        &mut self,
        parent: &ContainerVisual,
        g: &str,
        cx: f32,
        cy: f32,
        size: f32,
        rgb: [u8; 3],
    ) {
        let px = (size * self.dpi).round() as i32;
        let b = (size * 1.8 * self.dpi).round() as i32;
        let key = format!("ico|{px}|{rgb:?}|{g}");
        let hit = self.cache.get(&key).cloned();
        let (surf, w, h) = match hit {
            Some(v) => v,
            None => {
                let Some(bmp) = render_face(g, &Style { px, weight: 400 }, ICON_FACE, b, b) else {
                    return;
                };
                let Some((cd, cw, ch)) =
                    azr_core::symbols::crop_ink(&recolor(&bmp, rgb), bmp.w, bmp.h)
                else {
                    return;
                };
                let Ok(sf) = self.gfx.surface_from_bgra(cw, ch, &cd) else {
                    return;
                };
                let v = (sf, cw as i32, ch as i32);
                self.cache.insert(key, v.clone());
                v
            }
        };
        let make = || -> windows_core::Result<()> {
            let br = self.comp.CreateSurfaceBrushWithSurface(&surf)?;
            br.SetStretch(CompositionStretch::None)?;
            let v = self.comp.CreateSpriteVisual()?;
            v.SetBrush(&br)?;
            v.SetSize(Vector2 {
                X: w as f32,
                Y: h as f32,
            })?;
            v.SetOffset(Vector3 {
                X: cx - w as f32 / 2.0,
                Y: cy - h as f32 / 2.0,
                Z: 0.0,
            })?;
            parent.Children()?.InsertAtTop(&v)
        };
        let _ = make();
    }

    fn sprite(
        &self,
        parent: &ContainerVisual,
        s: &CompositionDrawingSurface,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        let make = || -> windows_core::Result<()> {
            let b = self.comp.CreateSurfaceBrushWithSurface(s)?;
            b.SetStretch(CompositionStretch::Fill)?;
            let v = self.comp.CreateSpriteVisual()?;
            v.SetBrush(&b)?;
            v.SetSize(Vector2 { X: w, Y: h })?;
            v.SetOffset(Vector3 { X: x, Y: y, Z: 0.0 })?;
            parent.Children()?.InsertAtTop(&v)
        };
        let _ = make();
    }

    fn thumb_surface(
        &mut self,
        key: String,
        w: u32,
        h: u32,
        make: impl FnOnce() -> Vec<u8>,
    ) -> Option<CompositionDrawingSurface> {
        if let Some(s) = self.thumbs.get(&key) {
            return Some(s.clone());
        }
        if self.thumbs.len() > 64 {
            self.thumbs.clear();
        }
        let s = self.gfx.surface_from_bgra(w, h, &make()).ok()?;
        self.thumbs.insert(key, s.clone());
        Some(s)
    }

    fn go(&mut self, t: u8, push: bool) {
        if push && t != self.tab {
            self.history.truncate(self.hpos + 1);
            self.history.push(t);
            self.hpos = self.history.len() - 1;
        }
        self.tab = t;
        self.scroll = 0.0;
        self.render();
    }

    fn page_matches(&self, idx: u8, name: &'static str, query: &str) -> bool {
        query.is_empty()
            || search_matches(name, query)
            || search_matches(prefs::tr(name), query)
            || self.page(idx).iter().any(|(heading, rows)| {
                heading.is_some_and(|h| search_matches(h, query))
                    || rows.iter().any(|r| search_matches(row_label(r), query))
            })
    }

    fn select_page(&mut self, page: u8) {
        self.search.clear();
        self.search_focus = false;
        self.side_scroll = 0.0;
        self.go(page, true);
    }

    fn scroll_content(&mut self, offset: f32) {
        let viewport_h = (H - VIEW_TOP - 10.0) * self.dpi;
        let next = offset.clamp(0.0, (self.content_h - viewport_h).max(0.0));
        if next == self.scroll {
            return;
        }
        self.scroll = next;
        if let Some(body) = &self.content_body {
            let _ = body.SetOffset(Vector3 {
                X: 0.0,
                Y: -self.scroll,
                Z: 0.0,
            });
        }
    }

    fn region(&mut self, x: f32, y: f32, w: f32, h: f32, a: Action) {
        self.regions.push(Region {
            x,
            y,
            w,
            h,
            a,
            scroll: true,
        });
    }

    fn hover_side(&mut self, hit: Option<u8>) {
        if hit == self.side_hover {
            return;
        }
        for (page, on) in [(self.side_hover, false), (hit, true)] {
            let Some((_, r, ic)) = self.side.iter().find(|e| Some(e.0) == page) else {
                continue;
            };
            let to = if on { crate::motion::HOVER_ROW } else { 1.0 };
            let _ = crate::motion::hover_pop(
                &self.comp,
                ic,
                Vector3 {
                    X: r.0 + r.2 / 2.0,
                    Y: r.1 + r.3 / 2.0,
                    Z: 0.0,
                },
                to,
            );
        }
        self.side_hover = hit;
    }

    fn region_fixed(&mut self, x: f32, y: f32, w: f32, h: f32, a: Action) {
        self.regions.push(Region {
            x,
            y,
            w,
            h,
            a,
            scroll: false,
        });
    }

    fn spec(id: u8) -> (f32, f32, &'static str) {
        match id {
            4 => (32.0, 72.0, " dp"),
            5 => (0.0, 24.0, " dp"),
            6 => (100.0, 250.0, "%"),
            7 => (0.0, 60.0, " dp"),
            8 => (70.0, 260.0, " dp"),
            9 => (0.0, 60.0, " dp"),
            10 => (2.0, 10.0, ""),
            11 => (50.0, 200.0, "%"),
            _ => (0.0, 100.0, "%"),
        }
    }

    fn commit_edit(&mut self) {
        let Some(f) = self.edit.take() else { return };
        let t = azr_core::settings::ShortText::new(&self.edit_buf);
        prefs::update(|s| {
            if f == 0 {
                s.greet_name = t;
            } else {
                s.greet_text = t;
            }
        });
        self.render();
    }

    fn measured(&self, s: &str, size: f32, weight: i32) -> f32 {
        measure(
            s,
            &Style {
                px: (size * self.dpi).round() as i32,
                weight,
            },
        )
        .0 as f32
            / self.dpi
    }

    fn wrapped(&self, s: &str, width: f32, size: f32, weight: i32) -> Vec<String> {
        azr_core::settings_layout::wrap_words(s, width.max(1.0), |t| self.measured(t, size, weight))
    }

    fn text_fit(
        &mut self,
        parent: &ContainerVisual,
        s: &str,
        x: f32,
        y: f32,
        width: f32,
        size: f32,
        weight: i32,
        rgb: [u8; 3],
        alpha: f32,
        anchor: u8,
    ) {
        let limit = width / self.dpi;
        if self.measured(s, size, weight) <= limit {
            self.text(parent, s, x, y, size, weight, rgb, alpha, anchor);
            return;
        }
        let mut value = s.to_owned();
        while !value.is_empty() && self.measured(&format!("{value}…"), size, weight) > limit {
            value.pop();
        }
        value.push('…');
        self.text(parent, &value, x, y, size, weight, rgb, alpha, anchor);
    }

    fn row_metrics(
        &self,
        r: &RowDef,
    ) -> (
        azr_core::settings_layout::RowLayout,
        Vec<String>,
        Vec<azr_core::settings_layout::ControlRect>,
        Vec<Vec<String>>,
    ) {
        use azr_core::settings_layout::{flow_controls, row_layout, RowLayout};
        let width = CONTENT_W - 32.0;
        let label = row_label(r);
        let label_width = self.measured(label, 13.5, 400);
        let wrap_width = match r {
            RowDef::Toggle(..) if label_width + 62.0 <= width => width - 62.0,
            RowDef::Slider(..) => width - 64.0,
            _ => width,
        };
        let lines = self.wrapped(label, wrap_width, 13.5, 400);
        let mut boxes = Vec::new();
        let mut captions = Vec::new();
        let mut control_width = width;
        let control_height = match r {
            RowDef::Toggle(..) => {
                control_width = 46.0;
                22.0
            }
            RowDef::Slider(..) => 30.0,
            RowDef::Brand | RowDef::Corners(_) => 102.0,
            RowDef::Text(..) | RowDef::Pick(..) | RowDef::Info(..) => 30.0,
            RowDef::Seg(_, opts, _) => {
                let widths: Vec<f32> = opts
                    .iter()
                    .map(|(s, _)| (self.measured(s, 12.5, 600) + 26.0).max(72.0))
                    .collect();
                boxes = flow_controls(width, &widths, 30.0, 6.0);
                boxes.last().map_or(30.0, |b| b.y + b.h)
            }
            RowDef::Thumbs(_, opts, _) => {
                let cell = width / opts.len().max(1) as f32;
                captions = opts
                    .iter()
                    .map(|(s, _, _)| self.wrapped(s, cell - 16.0, 11.5, 600))
                    .collect();
                let preview = if matches!(opts.first().map(|v| v.1), Some(Thumb::Icon(_))) {
                    56.0
                } else {
                    52.0
                };
                preview + 8.0 + captions.iter().map(Vec::len).max().unwrap_or(1) as f32 * 16.0
            }
            RowDef::Radios(opts, _) => {
                let widths: Vec<f32> = opts
                    .iter()
                    .map(|(s, _)| self.measured(s, 12.5, 500) + 42.0)
                    .collect();
                boxes = flow_controls(width, &widths, 30.0, 6.0);
                boxes.last().map_or(30.0, |b| b.y + b.h)
            }
            RowDef::Swatches(..) => 50.0,
        };
        let label_height = lines.len() as f32 * 18.0;
        let layout = if matches!(r, RowDef::Radios(..)) {
            RowLayout {
                label_y: 0.0,
                label_h: 0.0,
                control_x: 0.0,
                control_y: 12.0,
                control_w: width,
                height: control_height + 24.0,
                stacked: true,
            }
        } else {
            row_layout(
                width,
                label_width,
                control_width,
                label_height,
                control_height,
            )
        };
        (layout, lines, boxes, captions)
    }

    fn card(
        &mut self,
        c: &ContainerVisual,
        y0: f32,
        rows: &[RowDef],
        look: &azr_core::settings::Look,
    ) -> f32 {
        let d = self.dpi;
        let (x0, cw) = (CONTENT_X * d, CONTENT_W * d);
        let inner = CONTENT_W - 32.0;
        let metrics: Vec<_> = rows.iter().map(|r| self.row_metrics(r)).collect();
        let total = metrics.iter().map(|(m, _, _, _)| m.height * d).sum();
        self.rect(
            c,
            x0,
            y0,
            cw,
            total,
            12.0 * d,
            col(look.fg, if look.dark { 22 } else { 16 }),
        );
        let mut y = y0;
        let accent = prefs::accent();
        for (index, (r, (m, lines, boxes, captions))) in rows.iter().zip(metrics.iter()).enumerate()
        {
            let rh = m.height * d;
            let lx = x0 + 16.0 * d;
            let rx = x0 + cw - 16.0 * d;
            if index > 0 && !matches!(r, RowDef::Radios(..)) {
                self.rect(c, lx, y, cw - 32.0 * d, 1.0, 0.0, col(look.fg, 28));
            }
            if !matches!(r, RowDef::Radios(..)) {
                let limit = if matches!(r, RowDef::Slider(..)) {
                    inner - 64.0
                } else if !m.stacked && matches!(r, RowDef::Toggle(..)) {
                    inner - 62.0
                } else {
                    inner
                };
                for (i, line) in lines.iter().enumerate() {
                    self.text_fit(
                        c,
                        line,
                        lx,
                        y + (m.label_y + i as f32 * 18.0) * d,
                        limit * d,
                        13.5,
                        400,
                        look.fg,
                        0.95,
                        0,
                    );
                }
            }
            let cy = y + m.control_y * d;
            match r {
                RowDef::Toggle(_, on, act) => {
                    let (tw, th) = (46.0 * d, 22.0 * d);
                    let tx = rx - tw;
                    self.rect(
                        c,
                        tx,
                        cy,
                        tw,
                        th,
                        th / 2.0,
                        if *on {
                            col(accent, 255)
                        } else {
                            col(look.fg, 60)
                        },
                    );
                    let kh = 18.0 * d;
                    let kw = kh * 1.55;
                    let kx = if *on {
                        tx + tw - kw - 2.0 * d
                    } else {
                        tx + 2.0 * d
                    };
                    self.rect(
                        c,
                        kx,
                        cy + 2.0 * d,
                        kw,
                        kh,
                        kh / 2.0,
                        Color {
                            A: 255,
                            R: 255,
                            G: 255,
                            B: 255,
                        },
                    );
                    self.region(x0, y, cw, rh, *act);
                }
                RowDef::Seg(_, opts, selected) => {
                    for (i, ((text, act), b)) in opts.iter().zip(boxes).enumerate() {
                        let (bx, by, bw, bh) = (lx + b.x * d, cy + b.y * d, b.w * d, b.h * d);
                        let on = i == *selected;
                        self.rect(
                            c,
                            bx,
                            by,
                            bw,
                            bh,
                            10.0 * d,
                            col(if on { accent } else { look.fg }, if on { 255 } else { 25 }),
                        );
                        self.text_fit(
                            c,
                            text,
                            bx + bw / 2.0,
                            by + 7.0 * d,
                            bw - 20.0 * d,
                            12.5,
                            if on { 600 } else { 500 },
                            if on { [255, 255, 255] } else { look.fg },
                            1.0,
                            1,
                        );
                        self.region(bx, by, bw, bh, *act);
                    }
                }
                RowDef::Slider(_, value, id) => {
                    let (min, max, unit) = Self::spec(*id);
                    let frac = ((*value as f32 - min) / (max - min)).clamp(0.0, 1.0);
                    self.text_fit(
                        c,
                        &format!("{value}{unit}"),
                        rx,
                        y + m.label_y * d,
                        60.0 * d,
                        12.5,
                        500,
                        look.fg,
                        0.7,
                        2,
                    );
                    let tx = lx + 8.0 * d;
                    let tw = (inner - 16.0) * d;
                    let ty = cy + 14.0 * d;
                    let th = 5.0 * d;
                    self.rect(c, tx, ty, tw, th, th / 2.0, col(look.fg, 50));
                    self.rect(
                        c,
                        tx,
                        ty,
                        (tw * frac).max(th),
                        th,
                        th / 2.0,
                        col(accent, 255),
                    );
                    let kx = (tx + tw * frac - 8.0 * d).clamp(tx - 3.0 * d, tx + tw - 13.0 * d);
                    self.rect(
                        c,
                        kx - 1.0 * d,
                        ty - 7.0 * d,
                        18.0 * d,
                        18.0 * d,
                        9.0 * d,
                        col([0, 0, 0], 45),
                    );
                    self.rect(
                        c,
                        kx,
                        ty - 8.0 * d,
                        16.0 * d,
                        16.0 * d,
                        8.0 * d,
                        col([255, 255, 255], 255),
                    );
                    self.region(
                        tx - 6.0 * d,
                        cy,
                        tw + 12.0 * d,
                        30.0 * d,
                        Action::Slider(*id, tx, tw),
                    );
                }
                RowDef::Thumbs(_, opts, selected) => {
                    let cell = inner / opts.len().max(1) as f32;
                    let icon = matches!(opts.first().map(|o| o.1), Some(Thumb::Icon(_)));
                    let (tw, th) = if icon { (56.0, 56.0) } else { (80.0, 52.0) };
                    let state = prefs::get();
                    let dark = prefs::icon_dark();
                    for (i, ((_, kind, act), caption)) in opts.iter().zip(captions).enumerate() {
                        let on = i == *selected;
                        let center = lx + (i as f32 + 0.5) * cell * d;
                        let tx = center - tw * d / 2.0;
                        if on {
                            self.rect(
                                c,
                                tx - 4.0 * d,
                                cy - 4.0 * d,
                                (tw + 8.0) * d,
                                (th + 8.0) * d,
                                13.0 * d,
                                col(accent, 255),
                            );
                        }
                        let (pw, ph) = ((tw * d).round() as u32, (th * d).round() as u32);
                        let surface = match *kind {
                            Thumb::Appear(k) => self.thumb_surface(
                                format!("appearance|{k}|{pw}|{ph}"),
                                pw,
                                ph,
                                || azr_core::preview::appearance(k, pw, ph),
                            ),
                            Thumb::Icon(st) => {
                                let tint = state.icon_tint;
                                self.thumb_surface(
                                    format!("icon|{st:?}|{tint}|{dark}|{pw}"),
                                    pw,
                                    pw,
                                    || azr_core::preview::icon_tile(st, tint, dark, pw),
                                )
                            }
                        };
                        if let Some(sf) = surface {
                            self.sprite(c, &sf, tx, cy, tw * d, th * d);
                        }
                        for (line, text) in caption.iter().enumerate() {
                            self.text_fit(
                                c,
                                text,
                                center,
                                cy + (th + 8.0 + line as f32 * 16.0) * d,
                                (cell - 16.0) * d,
                                11.5,
                                if on { 600 } else { 400 },
                                look.fg,
                                if on { 1.0 } else { 0.7 },
                                1,
                            );
                        }
                        self.region(
                            lx + i as f32 * cell * d,
                            cy,
                            cell * d,
                            (m.height - m.control_y - 8.0) * d,
                            *act,
                        );
                    }
                }
                RowDef::Swatches(_, selected) => {
                    let (diam, gap) = (20.0 * d, 10.0 * d);
                    let n = azr_core::settings::ACCENTS.len() + 1;
                    let mut sx = lx;
                    for i in 0..n {
                        if i == 0 {
                            let px = diam.round() as u32;
                            if let Some(sf) =
                                self.thumb_surface(format!("multi|{px}"), px, px, || {
                                    azr_core::preview::multicolor(px)
                                })
                            {
                                self.sprite(c, &sf, sx, cy, diam, diam);
                            }
                        } else {
                            self.rect(
                                c,
                                sx,
                                cy,
                                diam,
                                diam,
                                diam / 2.0,
                                col(azr_core::settings::ACCENTS[i - 1], 255),
                            );
                        }
                        if i == *selected {
                            self.rect(
                                c,
                                sx + diam * 0.32,
                                cy + diam * 0.32,
                                diam * 0.36,
                                diam * 0.36,
                                diam * 0.18,
                                col([255, 255, 255], 255),
                            );
                        }
                        let value = if i == 0 {
                            azr_core::settings::ACCENT_MULTI
                        } else {
                            (i - 1) as u8
                        };
                        self.region(
                            sx - 3.0 * d,
                            cy - 3.0 * d,
                            diam + 6.0 * d,
                            diam + 6.0 * d,
                            Action::Accent(value),
                        );
                        sx += diam + gap;
                    }
                    let name = ACCENT_NAMES
                        .get(*selected)
                        .copied()
                        .unwrap_or(ACCENT_NAMES[0]);
                    self.text_fit(
                        c,
                        prefs::tr(name),
                        lx,
                        cy + 28.0 * d,
                        inner * d,
                        11.5,
                        500,
                        look.fg,
                        0.7,
                        0,
                    );
                }
                RowDef::Radios(opts, selected) => {
                    for (i, ((text, act), b)) in opts.iter().zip(boxes).enumerate() {
                        let (bx, by) = (lx + b.x * d, cy + b.y * d);
                        let on = i == *selected;
                        self.rect(
                            c,
                            bx,
                            by + 7.0 * d,
                            14.0 * d,
                            14.0 * d,
                            7.0 * d,
                            col(if on { accent } else { look.fg }, if on { 255 } else { 65 }),
                        );
                        if on {
                            self.rect(
                                c,
                                bx + 4.5 * d,
                                by + 11.5 * d,
                                5.0 * d,
                                5.0 * d,
                                2.5 * d,
                                col([255, 255, 255], 255),
                            );
                        }
                        if !on {
                            self.rect(
                                c,
                                bx + 1.5 * d,
                                by + 8.5 * d,
                                11.0 * d,
                                11.0 * d,
                                5.5 * d,
                                col(
                                    if look.dark {
                                        [44, 44, 48]
                                    } else {
                                        [244, 244, 246]
                                    },
                                    255,
                                ),
                            );
                        }
                        self.text_fit(
                            c,
                            text,
                            bx + 21.0 * d,
                            by + 6.0 * d,
                            (b.w - 28.0) * d,
                            12.5,
                            400,
                            look.fg,
                            0.95,
                            0,
                        );
                        self.region(bx, by, b.w * d, b.h * d, *act);
                    }
                }
                RowDef::Text(_, value, hint, field) => {
                    let editing = self.edit == Some(*field);
                    let shown = if editing {
                        format!("{}|", self.edit_buf)
                    } else if value.is_empty() {
                        hint.clone()
                    } else {
                        value.clone()
                    };
                    self.rect(
                        c,
                        lx,
                        cy,
                        inner * d,
                        30.0 * d,
                        8.0 * d,
                        col(
                            if editing { accent } else { look.fg },
                            if editing { 35 } else { 22 },
                        ),
                    );
                    self.text_fit(
                        c,
                        &shown,
                        lx + 12.0 * d,
                        cy + 7.0 * d,
                        (inner - 24.0) * d,
                        12.5,
                        500,
                        look.fg,
                        if !editing && value.is_empty() {
                            0.5
                        } else {
                            1.0
                        },
                        0,
                    );
                    self.region(lx, cy, inner * d, 30.0 * d, Action::EditText(*field));
                }
                RowDef::Brand => {
                    let size = (90.0 * d).round() as u32;
                    let key = format!("about-logo-{size}");
                    let surface = self.thumbs.get(&key).cloned().or_else(|| {
                        let image = crate::menulogo::original_bitmap(size)?;
                        let mut pixels = vec![0; (size * size * 4) as usize];
                        let offset = ((size - image.w) / 2, (size - image.h) / 2);
                        for (row, src) in image.bgra.chunks_exact(image.w as usize * 4).enumerate()
                        {
                            let start =
                                ((row + offset.1 as usize) * size as usize + offset.0 as usize) * 4;
                            pixels[start..start + src.len()].copy_from_slice(src);
                        }
                        self.thumb_surface(key, size, size, || pixels)
                    });
                    if let Some(surface) = surface {
                        let w = size as f32;
                        self.sprite(
                            c,
                            &surface,
                            lx + (inner * d - w) / 2.0,
                            cy + (102.0 * d - w) / 2.0,
                            w,
                            w,
                        );
                    }
                }
                RowDef::Corners(actions) => {
                    let w = 174.0 * d;
                    let h = 90.0 * d;
                    let x = lx + (inner * d - w) / 2.0;
                    let y = cy + 5.0 * d;
                    self.rect(c, x, y, w, h, 13.0 * d, col(look.fg, 40));
                    self.rect(
                        c,
                        x + 3.0 * d,
                        y + 3.0 * d,
                        w - 6.0 * d,
                        h - 6.0 * d,
                        10.0 * d,
                        col(look.fg, 16),
                    );
                    for (i, action) in actions.iter().enumerate() {
                        let px = if i % 2 == 0 {
                            x + 7.0 * d
                        } else {
                            x + w - 29.0 * d
                        };
                        let py = if i < 2 { y + 7.0 * d } else { y + h - 29.0 * d };
                        let color = if *action == azr_core::settings::HotAction::None {
                            col(look.fg, 70)
                        } else {
                            prefs::accent_c(235)
                        };
                        self.rect(c, px, py, 22.0 * d, 22.0 * d, 6.0 * d, color);
                        self.region(px, py, 22.0 * d, 22.0 * d, Action::HotCycle(i));
                    }
                    self.text(
                        c,
                        prefs::tr("Preview"),
                        x + w / 2.0,
                        y + 36.0 * d,
                        11.0,
                        600,
                        look.fg,
                        0.75,
                        1,
                    );
                }
                RowDef::Info(_, value) => {
                    self.text_fit(
                        c,
                        value,
                        lx,
                        cy + 7.0 * d,
                        inner * d,
                        13.5,
                        500,
                        look.fg,
                        1.0,
                        0,
                    );
                }
                RowDef::Pick(_, value, act) => {
                    self.rect(c, lx, cy, inner * d, 30.0 * d, 8.0 * d, col(look.fg, 25));
                    self.text_fit(
                        c,
                        value,
                        lx + 12.0 * d,
                        cy + 7.0 * d,
                        (inner - 48.0) * d,
                        12.5,
                        500,
                        look.fg,
                        1.0,
                        0,
                    );
                    self.text(
                        c,
                        "⌃",
                        rx - 16.0 * d,
                        cy + 8.0 * d,
                        11.0,
                        600,
                        look.fg,
                        0.6,
                        1,
                    );
                    self.region(lx, cy, inner * d, 30.0 * d, *act);
                }
            }
            y += rh;
        }
        total
    }

    fn page(&self, page: u8) -> Vec<(Option<&'static str>, Vec<RowDef>)> {
        let s = prefs::get();
        let behaviors = |dock: bool| -> Vec<(&'static str, Action)> {
            [
                (prefs::tr("Fixed"), Behavior::Fixed),
                (prefs::tr("Overlay"), Behavior::Overlay),
                (prefs::tr("Smart"), Behavior::Smart),
            ]
            .into_iter()
            .map(|(t, b)| {
                (
                    t,
                    if dock {
                        Action::Dock(b)
                    } else {
                        Action::Notch(b)
                    },
                )
            })
            .collect()
        };
        let beh_idx = |b: Behavior| match b {
            Behavior::Fixed => 0,
            Behavior::Overlay => 1,
            Behavior::Smart => 2,
        };
        match page {
            0 => vec![
                (
                    None,
                    vec![RowDef::Seg(
                        prefs::tr("Language"),
                        vec![
                            ("Tiếng Việt", Action::Lang(Lang::Vi)),
                            ("English", Action::Lang(Lang::En)),
                        ],
                        if s.lang == Lang::Vi { 0 } else { 1 },
                    )],
                ),
                (
                    Some(prefs::tr("System")),
                    vec![
                        RowDef::Toggle(
                            prefs::tr("Launch at Windows login"),
                            s.autostart,
                            Action::Autostart,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Hide the Windows taskbar"),
                            s.takeover,
                            Action::Takeover,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Hide desktop icons while Azr runs"),
                            s.hide_desktop_icons,
                            Action::HideIcons,
                        ),
                    ],
                ),
                (Some(prefs::tr("Display")), {
                    let selected = crate::display::selected().id;
                    let mut rows = vec![RowDef::Pick(
                        prefs::tr("Azr main display"),
                        if s.main_display == 0 {
                            format!("✓ {}", prefs::tr("Windows main display"))
                        } else {
                            prefs::tr("Windows main display").to_owned()
                        },
                        Action::MainDisplay(0),
                    )];
                    rows.extend(crate::display::list().into_iter().map(|display| {
                        let (w, h) = display.size();
                        let active = s.main_display != 0 && display.id == selected;
                        let marker = if active { "✓ " } else { "" };
                        RowDef::Pick(
                            prefs::tr(if display.primary {
                                "Windows primary"
                            } else {
                                "Secondary display"
                            }),
                            format!(
                                "{marker}{} {} · {w} × {h}",
                                prefs::tr("Display"),
                                display.id
                            ),
                            Action::MainDisplay(display.id),
                        )
                    }));
                    rows
                }),
                (
                    Some(prefs::tr("Shortcuts and Launchpad")),
                    vec![
                        RowDef::Toggle(
                            prefs::tr("Switch app windows with Alt+`"),
                            s.mac_shortcuts,
                            Action::MacKeys,
                        ),
                        RowDef::Seg(
                            prefs::tr("Launchpad shortcut"),
                            vec![
                                ("Ctrl+Shift+Space", Action::LaunchpadKey(0)),
                                (prefs::tr("Windows key"), Action::LaunchpadKey(1)),
                                ("Ctrl+Alt+L", Action::LaunchpadKey(2)),
                            ],
                            s.launchpad_key as usize,
                        ),
                        RowDef::Seg(
                            prefs::tr("Launchpad icon"),
                            vec![
                                (prefs::tr("App grid"), Action::LaunchpadIcon(0)),
                                ("Windows", Action::LaunchpadIcon(1)),
                                (prefs::tr("Finder style"), Action::LaunchpadIcon(2)),
                                (prefs::tr("Your image"), Action::LaunchpadIcon(3)),
                            ],
                            s.launchpad_icon as usize,
                        ),
                        RowDef::Pick(
                            prefs::tr("Logo image"),
                            prefs::tr(if crate::menulogo::launcher_path().is_some() {
                                "Replace image"
                            } else {
                                "Choose image"
                            })
                            .to_owned(),
                            Action::LaunchpadIcon(4),
                        ),
                    ],
                ),
                (
                    Some(prefs::tr("Windows")),
                    vec![
                        RowDef::Toggle(
                            prefs::tr("Azr minimize and restore effects"),
                            s.anim,
                            Action::Anim,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Soft shadow under the active window"),
                            s.win_shadow,
                            Action::WinShadow,
                        ),
                    ],
                ),
            ],
            1 => vec![
                (
                    None,
                    vec![
                        RowDef::Toggle(prefs::tr("Enable Dock"), s.dock_enabled, Action::DockOn),
                        RowDef::Seg(
                            prefs::tr("When to show Dock"),
                            behaviors(true),
                            beh_idx(s.dock),
                        ),
                        RowDef::Seg(
                            prefs::tr("Minimize effect"),
                            vec![
                                (prefs::tr("Funnel"), Action::MinEffect(0)),
                                (prefs::tr("Scale"), Action::MinEffect(1)),
                                ("Windows", Action::MinEffect(2)),
                            ],
                            s.min_effect as usize,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Uniform icons (squircle)"),
                            s.dock_plates,
                            Action::DockPlates,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Downloads stack"),
                            s.downloads_stack,
                            Action::DownloadsStack,
                        ),
                        RowDef::Toggle(prefs::tr("Recent apps"), s.recent_apps, Action::RecentApps),
                    ],
                ),
                (
                    Some(prefs::tr("Dock surface")),
                    vec![
                        RowDef::Seg(
                            prefs::tr("Surface style"),
                            vec![
                                ("Liquid Glass", Action::DockSurface(0)),
                                (prefs::tr("Frosted"), Action::DockSurface(1)),
                                (prefs::tr("Transparent"), Action::DockSurface(2)),
                                (prefs::tr("Solid"), Action::DockSurface(3)),
                            ],
                            s.dock_surface as usize,
                        ),
                        RowDef::Slider(prefs::tr("Background opacity"), s.dock_opacity, 12),
                        RowDef::Slider(prefs::tr("Glass blur"), s.dock_blur, 13),
                        RowDef::Toggle(
                            prefs::tr("Glass edge refraction (Dock)"),
                            s.refraction,
                            Action::Refraction,
                        ),
                        RowDef::Seg(
                            prefs::tr("Icon backplate"),
                            vec![
                                (prefs::tr("Original"), Action::IconBackground(0)),
                                (prefs::tr("Glass"), Action::IconBackground(1)),
                                (prefs::tr("None"), Action::IconBackground(2)),
                            ],
                            s.icon_background as usize,
                        ),
                        RowDef::Slider(prefs::tr("Icon corner roundness"), s.icon_round, 14),
                        RowDef::Pick(
                            prefs::tr("macOS-style icon frame"),
                            prefs::tr("Apply").to_owned(),
                            Action::MacIconCorners,
                        ),
                    ],
                ),
                (
                    Some(prefs::tr("Layout")),
                    vec![
                        RowDef::Slider(prefs::tr("Icon size"), s.dock_size, 4),
                        RowDef::Slider(prefs::tr("Icon spacing"), s.dock_gap, 5),
                        RowDef::Slider(prefs::tr("Hover zoom"), s.dock_zoom, 6),
                        RowDef::Slider(prefs::tr("Distance from bottom edge"), s.dock_margin, 7),
                        RowDef::Slider(prefs::tr("Dock corner roundness"), s.dock_round, 2),
                    ],
                ),
            ],
            2 => vec![
                (
                    None,
                    vec![
                        RowDef::Toggle(
                            prefs::tr("Enable floating panel"),
                            s.notch_enabled,
                            Action::NotchOn,
                        ),
                        RowDef::Seg(
                            prefs::tr("Shape"),
                            vec![
                                (prefs::tr("Floating panel"), Action::NotchShape(false)),
                                (prefs::tr("Capsule"), Action::NotchShape(true)),
                            ],
                            if s.notch_island { 1 } else { 0 },
                        ),
                        RowDef::Seg(prefs::tr("Behavior"), behaviors(false), beh_idx(s.notch)),
                        RowDef::Slider(prefs::tr("Corner roundness"), s.notch_round, 3),
                        RowDef::Toggle(
                            prefs::tr("Open when the cursor is on the floating panel"),
                            s.notch_hover_open,
                            Action::HoverOpen,
                        ),
                    ],
                ),
                (Some(prefs::tr("When the floating panel is idle")), {
                    let name = if s.greet_name.is_empty() {
                        user_name()
                    } else {
                        s.greet_name.as_str().to_string()
                    };
                    let hour = azr_platform::time::local_now().hour;
                    let auto = azr_core::notch::glance::greeting_text("", &name, hour, s.lang);
                    vec![
                        RowDef::Seg(
                            prefs::tr("Show"),
                            vec![
                                (prefs::tr("Greeting & info"), Action::NotchIdle(0)),
                                (prefs::tr("Music & lyrics"), Action::NotchIdle(2)),
                                (prefs::tr("Nothing"), Action::NotchIdle(1)),
                            ],
                            match s.notch_idle {
                                0 => 0,
                                2 => 1,
                                _ => 2,
                            },
                        ),
                        RowDef::Text(
                            prefs::tr("Nickname"),
                            s.greet_name.as_str().to_string(),
                            user_name(),
                            0,
                        ),
                        RowDef::Text(
                            prefs::tr("Greeting text"),
                            s.greet_text.as_str().to_string(),
                            auto,
                            1,
                        ),
                    ]
                }),
                (
                    Some(prefs::tr("Lyrics")),
                    vec![
                        RowDef::Toggle(
                            prefs::tr("Show lyrics in the floating panel"),
                            s.lyrics_compact,
                            Action::LyricsCompact,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Look up lyrics online (LRCLIB)"),
                            s.lyrics_online,
                            Action::LyricsOnline,
                        ),
                        RowDef::Pick(
                            prefs::tr("Lyrics folder (.lrc)"),
                            prefs::tr("Open").to_string(),
                            Action::LyricsDir,
                        ),
                    ],
                ),
                (
                    Some(prefs::tr("Activities")),
                    vec![
                        RowDef::Toggle(
                            prefs::tr("Show downloads (Store, Steam, browsers)"),
                            s.notch_downloads,
                            Action::NotchDownloads,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Send via QR (LAN HTTP, unencrypted, 10 min)"),
                            s.notch_send,
                            Action::NotchSend,
                        ),
                    ],
                ),
            ],
            3 => {
                let cur = if s.menubar_material {
                    3
                } else if !s.menubar_glass {
                    0
                } else {
                    match s.menubar_style {
                        azr_core::settings::BarStyle::Light => 1,
                        azr_core::settings::BarStyle::Dark => 2,
                    }
                };
                vec![
                    (
                        None,
                        vec![
                            RowDef::Toggle(
                                prefs::tr("Enable menu bar"),
                                s.menubar,
                                Action::MenuBarOn,
                            ),
                            RowDef::Seg(
                                prefs::tr("menu bar background"),
                                vec![
                                    (prefs::tr("Transparent"), Action::BarStyle(0)),
                                    (prefs::tr("Light"), Action::BarStyle(1)),
                                    (prefs::tr("Dark"), Action::BarStyle(2)),
                                    (prefs::tr("Material"), Action::BarStyle(3)),
                                ],
                                cur,
                            ),
                            RowDef::Toggle(
                                prefs::tr("24-hour clock"),
                                s.clock_24h,
                                Action::Clock24,
                            ),
                            RowDef::Seg(
                                prefs::tr("Battery percentage"),
                                vec![
                                    (prefs::tr("Inside"), Action::BatteryPct(1)),
                                    (prefs::tr("Outside"), Action::BatteryPct(2)),
                                    (prefs::tr("Hidden"), Action::BatteryPct(0)),
                                ],
                                match s.battery_pct {
                                    1 => 0,
                                    2 => 1,
                                    _ => 2,
                                },
                            ),
                        ],
                    ),
                    (Some(prefs::tr("Menu logo")), {
                        let custom = crate::menulogo::path().is_some();
                        let mut rows = vec![
                            RowDef::Seg(
                                prefs::tr("Menu logo"),
                                vec![
                                    (prefs::tr("Ring"), Action::MenuLogoMode(0)),
                                    ("Azr", Action::MenuLogoMode(1)),
                                    (prefs::tr("Your image"), Action::MenuLogoMode(2)),
                                ],
                                s.menu_logo as usize,
                            ),
                            RowDef::Pick(
                                prefs::tr("Logo image"),
                                prefs::tr(if custom {
                                    "Replace image"
                                } else {
                                    "Choose image"
                                })
                                .to_owned(),
                                Action::MenuLogoPick,
                            ),
                        ];
                        if custom {
                            rows.push(RowDef::Pick(
                                prefs::tr("Custom logo"),
                                prefs::tr("Remove image").to_owned(),
                                Action::MenuLogoReset,
                            ));
                        }
                        rows
                    }),
                ]
            }
            4 => {
                use azr_core::settings::IconStyle;
                let theme_idx = match s.theme {
                    Theme::Auto => 0,
                    Theme::Light => 1,
                    Theme::Dark => 2,
                };
                let mat_idx = |m: Material| Material::ALL.iter().position(|x| *x == m).unwrap_or(0);
                let mat_opts = |dock: bool| {
                    vec![
                        (prefs::tr("Blur"), Action::Material(dock, Material::Blur)),
                        (
                            prefs::tr("Crystal"),
                            Action::Material(dock, Material::Glass),
                        ),
                        (
                            prefs::tr("Black blur"),
                            Action::Material(dock, Material::Black),
                        ),
                    ]
                };
                let ist = match s.icon_style {
                    IconStyle::Default => 0,
                    IconStyle::Dark => 1,
                    IconStyle::Clear => 2,
                    IconStyle::Tinted => 3,
                };
                let accent_idx = if s.accent == azr_core::settings::ACCENT_MULTI {
                    0
                } else {
                    s.accent as usize + 1
                };
                let mut theme = vec![
                    RowDef::Swatches(prefs::tr("Color"), accent_idx),
                    RowDef::Thumbs(
                        prefs::tr("Icon & widget style"),
                        vec![
                            (
                                prefs::tr("Default"),
                                Thumb::Icon(IconStyle::Default),
                                Action::IconStyle(0),
                            ),
                            (
                                prefs::tr("Dark"),
                                Thumb::Icon(IconStyle::Dark),
                                Action::IconStyle(1),
                            ),
                            (
                                prefs::tr("Clear icons"),
                                Thumb::Icon(IconStyle::Clear),
                                Action::IconStyle(2),
                            ),
                            (
                                prefs::tr("Tinted"),
                                Thumb::Icon(IconStyle::Tinted),
                                Action::IconStyle(3),
                            ),
                        ],
                        ist,
                    ),
                ];
                if s.icon_style != IconStyle::Default {
                    let v = match s.icon_variant {
                        1 => 0,
                        2 => 1,
                        _ => 2,
                    };
                    theme.push(RowDef::Radios(
                        vec![
                            (prefs::tr("Light"), Action::IconVariant(1)),
                            (prefs::tr("Dark"), Action::IconVariant(2)),
                            (prefs::tr("Automatic"), Action::IconVariant(0)),
                        ],
                        v,
                    ));
                }
                if s.icon_style == IconStyle::Tinted {
                    theme.push(RowDef::Seg(
                        prefs::tr("Tint color"),
                        vec![
                            (prefs::tr("Blue"), Action::IconTint(0)),
                            (prefs::tr("Purple"), Action::IconTint(1)),
                            (prefs::tr("Pink"), Action::IconTint(2)),
                            (prefs::tr("Orange"), Action::IconTint(3)),
                            (prefs::tr("Green"), Action::IconTint(4)),
                        ],
                        s.icon_tint as usize,
                    ));
                }
                theme.push(RowDef::Toggle(
                    prefs::tr("Icon pack (iconpack folder)"),
                    s.icon_pack,
                    Action::IconPack,
                ));
                let scale_idx = match s.ui_scale {
                    0..=112 => 0,
                    113..=137 => 1,
                    _ => 2,
                };
                vec![
                    (
                        None,
                        vec![RowDef::Seg(
                            prefs::tr("Dock, floating panel, menu bar & Quick Panel scale"),
                            vec![
                                ("100%", Action::UiScale(100)),
                                ("125%", Action::UiScale(125)),
                                ("150%", Action::UiScale(150)),
                            ],
                            scale_idx,
                        )],
                    ),
                    (
                        None,
                        vec![RowDef::Thumbs(
                            prefs::tr("Appearance"),
                            vec![
                                (
                                    prefs::tr("Automatic"),
                                    Thumb::Appear(0),
                                    Action::Theme(Theme::Auto),
                                ),
                                (
                                    prefs::tr("Light"),
                                    Thumb::Appear(1),
                                    Action::Theme(Theme::Light),
                                ),
                                (
                                    prefs::tr("Dark"),
                                    Thumb::Appear(2),
                                    Action::Theme(Theme::Dark),
                                ),
                            ],
                            theme_idx,
                        )],
                    ),
                    (Some(prefs::tr("Theme")), theme),
                    (
                        Some(prefs::tr("Glass")),
                        vec![
                            RowDef::Seg(
                                prefs::tr("Material"),
                                mat_opts(true),
                                mat_idx(s.dock_material),
                            ),
                            RowDef::Seg(
                                prefs::tr("floating panel material"),
                                mat_opts(false),
                                mat_idx(s.notch_material),
                            ),
                            RowDef::Slider(prefs::tr("Blur level"), s.blur, 0),
                            RowDef::Slider(prefs::tr("Transparency"), s.transparency, 1),
                            RowDef::Toggle(
                                prefs::tr("Glass edge refraction (Dock)"),
                                s.refraction,
                                Action::Refraction,
                            ),
                        ],
                    ),
                ]
            }
            5 => vec![
                (
                    None,
                    vec![
                        RowDef::Toggle(
                            prefs::tr("Enable window manager"),
                            s.stage,
                            Action::StageOn,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Open Overview with Alt+Tab"),
                            s.overview_alt_tab,
                            Action::SwitcherOn,
                        ),
                        RowDef::Toggle(
                            prefs::tr("Click wallpaper to hide all windows"),
                            s.stage_desktop_click,
                            Action::StageFlag(1),
                        ),
                        RowDef::Toggle(
                            prefs::tr("Show window title on hover"),
                            s.stage_titles,
                            Action::StageFlag(3),
                        ),
                        RowDef::Toggle(
                            prefs::tr("Pause media of background apps"),
                            s.stage_media,
                            Action::StageFlag(2),
                        ),
                    ],
                ),
                (
                    Some(prefs::tr("Window list")),
                    vec![
                        RowDef::Seg(
                            prefs::tr("Window list position"),
                            vec![
                                (prefs::tr("Left"), Action::StageRight(false)),
                                (prefs::tr("Right"), Action::StageRight(true)),
                            ],
                            if s.stage_right { 1 } else { 0 },
                        ),
                        RowDef::Seg(
                            prefs::tr("Desktop icons"),
                            vec![
                                (prefs::tr("Hidden"), Action::StageIcons(0)),
                                (prefs::tr("Automatic"), Action::StageIcons(1)),
                                (prefs::tr("Shown"), Action::StageIcons(2)),
                            ],
                            s.stage_icons as usize,
                        ),
                        RowDef::Slider(prefs::tr("Card size"), s.stage_size, 8),
                        RowDef::Slider(prefs::tr("Card spacing"), s.stage_gap, 9),
                        RowDef::Slider(prefs::tr("Number of cards"), s.stage_count, 10),
                        RowDef::Slider(prefs::tr("Animation speed"), s.stage_speed, 11),
                    ],
                ),
            ],
            6 => {
                let names = ["Top left", "Top right", "Bottom left", "Bottom right"];
                let mut rows = vec![RowDef::Corners(s.hot)];
                rows.extend(names.iter().enumerate().map(|(i, name)| {
                    RowDef::Pick(
                        prefs::tr(name),
                        prefs::tr(hot_label(s.hot[i])).to_string(),
                        Action::HotCycle(i),
                    )
                }));
                vec![(None, rows)]
            }
            8 => vec![(
                None,
                vec![
                    RowDef::Seg(
                        prefs::tr("Widget material"),
                        vec![
                            (prefs::tr("Original"), Action::WidgetStyle(0)),
                            (prefs::tr("Crystal"), Action::WidgetStyle(1)),
                        ],
                        s.widget_style.min(1) as usize,
                    ),
                    RowDef::Pick(
                        prefs::tr("Desktop widgets"),
                        prefs::tr("Add widgets").to_string(),
                        Action::WidgetGallery,
                    ),
                ],
            )],
            9 => vec![
                (
                    Some(prefs::tr("About Azr")),
                    vec![
                        RowDef::Brand,
                        RowDef::Info(prefs::tr("Version"), env!("CARGO_PKG_VERSION")),
                        RowDef::Info(prefs::tr("Developer"), "Ericismee"),
                        RowDef::Pick(
                            prefs::tr("Source repository"),
                            "GitHub - Ericismee/Azr".into(),
                            Action::ProjectRepo,
                        ),
                    ],
                ),
                (
                    Some(prefs::tr("Support the developer")),
                    vec![RowDef::Pick(
                        prefs::tr("Support Azr"),
                        prefs::tr("Open support website").into(),
                        Action::ProjectSupport,
                    )],
                ),
            ],
            _ => vec![(
                None,
                vec![RowDef::Toggle(
                    prefs::tr("Clipboard history in Quick Search"),
                    s.clipboard_history,
                    Action::ClipHistory,
                )],
            )],
        }
    }

    fn render(&mut self) {
        let new_dpi = fitted_dpi(self.popup.hwnd);
        if (new_dpi - self.dpi).abs() > 0.001 {
            let ratio = new_dpi / self.dpi;
            self.scroll *= ratio;
            self.side_scroll *= ratio;
            self.dpi = new_dpi;
        }
        self.content_body = None;
        let content = self.popup.content.clone();
        if let Ok(k) = content.Children() {
            let _ = k.RemoveAll();
        }
        self.regions.clear();
        let d = self.dpi;
        let look = prefs::look(Role::Themed);
        let c = &content;

        for (i, rgb) in [[255u8, 95, 87], [254, 188, 46], [40, 200, 64]]
            .iter()
            .enumerate()
        {
            let (cx, cy) = ((26.0 + i as f32 * 20.0) * d, 27.0 * d);
            self.rect(
                c,
                cx - 6.0 * d,
                cy - 6.0 * d,
                12.0 * d,
                12.0 * d,
                6.0 * d,
                col(*rgb, 255),
            );
        }
        self.region_fixed(18.0 * d, 17.0 * d, 17.0 * d, 20.0 * d, Action::Close);
        self.region_fixed(37.0 * d, 17.0 * d, 17.0 * d, 20.0 * d, Action::Minimize);
        self.text(
            c,
            prefs::tr("Settings"),
            84.0 * d,
            19.0 * d,
            13.0,
            700,
            look.fg,
            0.94,
            0,
        );

        let (fx, fy, fw, fh) = (16.0 * d, 48.0 * d, (SIDEBAR_W - 32.0) * d, 30.0 * d);
        self.rect(
            c,
            fx,
            fy,
            fw,
            fh,
            9.0 * d,
            col(look.fg, if self.search_focus { 36 } else { 22 }),
        );
        self.glyph_icon(c, "\u{E721}", fx + 15.0 * d, fy + fh / 2.0, 11.0, look.fg);
        let (shown, op) = if self.search.is_empty() && !self.search_focus {
            (prefs::tr("Search settings").to_string(), 0.45)
        } else {
            (
                format!(
                    "{}{}",
                    self.search,
                    if self.search_focus { "|" } else { "" }
                ),
                0.95,
            )
        };
        self.text_fit(
            c,
            &shown,
            fx + 29.0 * d,
            fy + (fh - 17.0 * d) / 2.0,
            fw - if self.search.is_empty() {
                38.0 * d
            } else {
                58.0 * d
            },
            13.0,
            400,
            look.fg,
            op,
            0,
        );
        self.region_fixed(fx, fy, fw, fh, Action::Search);

        if !self.search.is_empty() {
            let clear_x = fx + fw - 24.0 * d;
            self.glyph_icon(
                c,
                "\u{E711}",
                clear_x + 9.0 * d,
                fy + fh / 2.0,
                9.0,
                look.fg,
            );
            self.region_fixed(clear_x, fy, 24.0 * d, fh, Action::ClearSearch);
        }

        let query = fold_search(self.search.trim());
        let mut sy = (SIDE_TOP + 4.0) * d;
        self.side.clear();
        let (view_top, view_bot) = (SIDE_TOP * d, SIDE_BOTTOM * d);
        let side_body = match (
            self.comp.CreateContainerVisual(),
            self.comp.CreateContainerVisual(),
        ) {
            (Ok(v), Ok(b)) => {
                let _ = v.SetOffset(Vector3 {
                    X: 0.0,
                    Y: view_top,
                    Z: 0.0,
                });
                let _ = v.SetSize(Vector2 {
                    X: SIDEBAR_W * d,
                    Y: view_bot - view_top,
                });
                if let Ok(clip) = self.comp.CreateInsetClip() {
                    let _ = v.SetClip(&clip);
                }
                let _ = b.SetOffset(Vector3 {
                    X: 0.0,
                    Y: -view_top - self.side_scroll,
                    Z: 0.0,
                });
                let _ = c.Children().and_then(|k| k.InsertAtTop(&v));
                let _ = v.Children().and_then(|k| k.InsertAtTop(&b));
                b
            }
            _ => c.clone(),
        };
        let parent = &side_body;
        for (name, glyph, color, idx) in PAGES {
            if !self.page_matches(idx, name, &query) {
                continue;
            }
            let sel = idx == self.tab;
            let (rx, rw, rh) = (16.0 * d, (SIDEBAR_W - 32.0) * d, 36.0 * d);
            let Ok(ic) = self.comp.CreateContainerVisual() else {
                continue;
            };
            let _ = parent.Children().and_then(|k| k.InsertAtTop(&ic));
            if self.side_hover == Some(idx) {
                let _ = ic.SetCenterPoint(Vector3 {
                    X: rx + rw / 2.0,
                    Y: sy + rh / 2.0,
                    Z: 0.0,
                });
                let _ = ic.SetScale(Vector3 {
                    X: crate::motion::HOVER_ROW,
                    Y: crate::motion::HOVER_ROW,
                    Z: 1.0,
                });
            }
            self.side.push((idx, (rx, sy, rw, rh), ic.clone()));
            let c = &ic;
            if sel {
                self.rect(c, rx, sy, rw, rh, 10.0 * d, crate::prefs::accent_c(235));
            }
            self.rect(
                c,
                rx + 8.0 * d,
                sy + 6.0 * d,
                24.0 * d,
                24.0 * d,
                7.0 * d,
                col(color, 255),
            );
            self.glyph_icon(
                c,
                glyph,
                rx + 20.0 * d,
                sy + 18.0 * d,
                13.0,
                [255, 255, 255],
            );
            self.text_fit(
                c,
                prefs::tr(name),
                rx + 42.0 * d,
                sy + (rh - 17.0 * d) / 2.0,
                rw - 50.0 * d,
                13.0,
                if sel { 600 } else { 400 },
                if sel { [255, 255, 255] } else { look.fg },
                1.0,
                0,
            );
            let vis_top = (sy - self.side_scroll).max(view_top);
            let vis_bot = (sy - self.side_scroll + rh).min(view_bot);
            if vis_bot > vis_top {
                self.region_fixed(rx, vis_top, rw, vis_bot - vis_top, Action::Tab(idx));
            }
            sy += 40.0 * d;
        }
        if self.side.is_empty() {
            self.text_fit(
                c,
                prefs::tr("No matching settings"),
                20.0 * d,
                sy + 8.0 * d,
                (SIDEBAR_W - 40.0) * d,
                12.0,
                500,
                look.fg,
                0.70,
                0,
            );
        }
        let side_max = (sy - 4.0 * d - view_bot).max(0.0);
        self.side_view = (view_top, side_max);
        if self.side_scroll > side_max {
            self.side_scroll = side_max;
        }
        self.side_bar = None;
        if side_max > 0.5 {
            let (track_top, track_h) = (view_top + 4.0 * d, view_bot - view_top - 8.0 * d);
            let content = view_bot - view_top + side_max;
            let th = (track_h * (view_bot - view_top) / content).max(28.0 * d);
            let ty = track_top + (track_h - th) * (self.side_scroll / side_max);
            let bw = if self.side_drag.is_some() {
                7.0 * d
            } else {
                5.0 * d
            };
            let bx = (SIDEBAR_W - SIDE_INSET - 5.0) * d - bw;
            self.rect(
                c,
                bx,
                ty,
                bw,
                th,
                bw / 2.0,
                col(look.fg, if self.side_drag.is_some() { 110 } else { 70 }),
            );
            self.side_bar = Some(((bx - 6.0 * d, track_top, bw + 12.0 * d, track_h), th));
        }
        self.text(
            c,
            "Azr",
            SIDEBAR_W * d / 2.0,
            (H - 39.0) * d,
            12.0,
            600,
            look.fg,
            0.65,
            1,
        );

        self.text(
            c,
            concat!("v", env!("CARGO_PKG_VERSION")),
            SIDEBAR_W * d / 2.0,
            (H - 25.0) * d,
            11.0,
            500,
            look.fg,
            0.65,
            1,
        );
        self.region_fixed(
            14.0 * d,
            (H - 47.0) * d,
            (SIDEBAR_W - 28.0) * d,
            40.0 * d,
            Action::Tab(9),
        );

        let (bx, by, bw, bh) = (CONTENT_X * d, 11.0 * d, 64.0 * d, 28.0 * d);
        self.rect(c, bx, by, bw, bh, 14.0 * d, col(look.fg, 24));
        let (can_back, can_fwd) = (self.hpos > 0, self.hpos + 1 < self.history.len());
        let dim = |on: bool| {
            if on {
                look.fg
            } else if look.dark {
                [110, 110, 116]
            } else {
                [170, 170, 176]
            }
        };
        self.glyph_icon(
            c,
            "\u{E76B}",
            bx + bw * 0.27,
            by + bh / 2.0,
            11.0,
            dim(can_back),
        );
        self.glyph_icon(
            c,
            "\u{E76C}",
            bx + bw * 0.73,
            by + bh / 2.0,
            11.0,
            dim(can_fwd),
        );
        self.rect(
            c,
            bx + bw / 2.0,
            by + 7.0 * d,
            1.0,
            bh - 14.0 * d,
            0.0,
            col(look.fg, 34),
        );
        self.region_fixed(bx, by, bw / 2.0, bh, Action::Back);
        self.region_fixed(bx + bw / 2.0, by, bw / 2.0, bh, Action::Forward);
        let title = PAGES
            .iter()
            .find(|p| p.3 == self.tab)
            .map(|p| prefs::tr(p.0))
            .unwrap_or("");
        self.text(
            c,
            title,
            bx + bw + 12.0 * d,
            by + (bh - 19.0 * d) / 2.0,
            15.0,
            700,
            look.fg,
            1.0,
            0,
        );

        let view = match (
            self.comp.CreateContainerVisual(),
            self.comp.CreateContainerVisual(),
        ) {
            (Ok(v), Ok(b)) => Some((v, b)),
            _ => None,
        };
        let Some((viewport, body)) = view else { return };
        let (vt, vh) = (VIEW_TOP * d, (H - VIEW_TOP - 10.0) * d);
        let _ = viewport.SetOffset(Vector3 {
            X: 0.0,
            Y: vt,
            Z: 0.0,
        });
        let _ = viewport.SetSize(Vector2 { X: W * d, Y: vh });
        if let Ok(clip) = self.comp.CreateInsetClip() {
            let _ = clip.SetLeftInset(0.0);
            let _ = clip.SetRightInset(0.0);
            let _ = clip.SetTopInset(0.0);
            let _ = clip.SetBottomInset(0.0);
            let _ = viewport.SetClip(&clip);
        }
        let _ = body.SetOffset(Vector3 {
            X: 0.0,
            Y: -self.scroll,
            Z: 0.0,
        });
        let _ = c.Children().and_then(|k| k.InsertAtTop(&viewport));
        let _ = viewport.Children().and_then(|k| k.InsertAtTop(&body));

        let mut y = VIEW_TOP * d - vt;
        y += 4.0 * d;
        let groups = self.page(self.tab);
        for (header, rows) in groups {
            if let Some(h) = header {
                self.text_abs(
                    &body,
                    h,
                    CONTENT_X * d + 8.0 * d,
                    y + 2.0 * d,
                    12.0,
                    600,
                    look.fg,
                    0.55,
                );
                y += 24.0 * d;
            }
            let used = self.card_in(&body, y, &rows, &look, vt);
            y += used + 20.0 * d;
        }
        self.content_h = y + 6.0 * d;
        self.content_body = Some(body);
        self.scroll_content(self.scroll);
        self.size = (W * d, H * d);
        let (x, yy, ww, hh) = self.popup.rect;
        let (nw, nh) = ((W * d) as i32, (H * d) as i32);
        if (nw, nh) != (ww, hh) {
            let area = settings_work_area(self.popup.hwnd);
            let _ = self.popup.set_rect(
                x.clamp(area.left + 12, (area.right - nw - 12).max(area.left + 12)),
                yy.clamp(area.top + 12, (area.bottom - nh - 12).max(area.top + 12)),
                nw,
                nh,
            );
        }

        self.frame();
    }

    fn frame(&mut self) {
        let d = self.dpi;
        let (w, h) = ((W * d) as i32, (H * d) as i32);
        let (hx, hy, hw, hh) = (
            SIDE_INSET * d,
            SIDE_INSET * d,
            (SIDEBAR_W - 2.0 * SIDE_INSET) * d,
            (H - 2.0 * SIDE_INSET) * d,
        );
        let _ = self.popup.panel.set_rect(hx, hy, hw, hh);
        let _ = self.popup.panel.set_radius(SIDE_RADIUS * d);
        let dark = prefs::look(Role::Themed).dark;
        if self.frame.is_some() && self.frame_key == (w, h, dark) {
            return;
        }
        let data = frame_pixels(w, h, 24.0 * d, (hx, hy, hw, hh), SIDE_RADIUS * d, dark);
        let go = || -> windows_core::Result<windows::UI::Composition::SpriteVisual> {
            let surf = self.gfx.surface_from_bgra(w as u32, h as u32, &data)?;
            let br = self.comp.CreateSurfaceBrushWithSurface(&surf)?;
            br.SetStretch(CompositionStretch::Fill)?;
            let v = match &self.frame {
                Some(v) => v.clone(),
                None => {
                    let v = self.comp.CreateSpriteVisual()?;
                    self.popup.root().Children()?.InsertAtBottom(&v)?;
                    v
                }
            };
            v.SetBrush(&br)?;
            v.SetSize(Vector2 {
                X: w as f32,
                Y: h as f32,
            })?;
            Ok(v)
        };
        match go() {
            Ok(v) => {
                self.frame = Some(v);
                self.frame_key = (w, h, dark);
            }
            Err(e) => log::line("settings", &format!("bingkai gagal: {e}")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text_abs(
        &mut self,
        parent: &ContainerVisual,
        s: &str,
        x: f32,
        y: f32,
        size: f32,
        weight: i32,
        rgb: [u8; 3],
        alpha: f32,
    ) {
        self.text(parent, s, x, y, size, weight, rgb, alpha, 0);
    }

    fn card_in(
        &mut self,
        body: &ContainerVisual,
        y: f32,
        rows: &[RowDef],
        look: &azr_core::settings::Look,
        vt: f32,
    ) -> f32 {
        let before = self.regions.len();
        let used = self.card(body, y, rows, look);
        for r in self.regions[before..].iter_mut() {
            r.y += vt;
        }
        used
    }

    fn hit(&self, x: f32, y: f32) -> Option<Action> {
        let d = self.dpi;
        let (vt, vb) = (VIEW_TOP * d, (H - 10.0) * d);
        self.regions
            .iter()
            .rev()
            .find(|r| {
                let yy = if r.scroll {
                    if y < vt || y > vb {
                        return false;
                    }
                    y + self.scroll
                } else {
                    y
                };
                x >= r.x && x < r.x + r.w && yy >= r.y && yy < r.y + r.h
            })
            .map(|r| r.a)
    }

    fn slide(&self, id: u8, x: f32, sx: f32, sw: f32, save: bool) {
        let (min, max, _) = Self::spec(id);
        let v = (min + slider::value_at(x, sx, sw, false) * (max - min)).round() as u8;
        prefs::update_with(save, |s| match id {
            0 => s.blur = v,
            1 => s.transparency = v,
            2 => s.dock_round = v,
            3 => s.notch_round = v,
            4 => s.dock_size = v,
            5 => s.dock_gap = v,
            6 => s.dock_zoom = v,
            8 => s.stage_size = v,
            9 => s.stage_gap = v,
            10 => s.stage_count = v,
            11 => s.stage_speed = v,
            12 => s.dock_opacity = v,
            13 => s.dock_blur = v,
            14 => s.icon_round = v,
            _ => s.dock_margin = v,
        });
    }
}

fn hot_label(a: azr_core::settings::HotAction) -> &'static str {
    use azr_core::settings::HotAction::*;
    match a {
        None => "None",
        Mission => "Overview",
        Launchpad => "Apps",
        Desktop => "Show Desktop",
        Stage => "Window manager",
        Notifications => "Notification Center",
        Lock => "Lock Screen",
    }
}

fn xy(lp: LPARAM) -> (f32, f32) {
    ((lp.0 as i16) as f32, ((lp.0 >> 16) as i16) as f32)
}

fn with_win<R>(f: impl FnOnce(&mut Win) -> R) -> Option<R> {
    WIN.with(|s| {
        s.try_borrow_mut()
            .ok()
            .and_then(|mut b| b.as_mut().map(|w| f(w)))
    })
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            let mut r = windows::Win32::Foundation::RECT::default();
            let _ = GetWindowRect(hwnd, &mut r);
            let (px, py) = ((lp.0 as i16) as i32, ((lp.0 >> 16) as i16) as i32);
            let drag = with_win(|w| {
                let d = w.dpi;
                py - r.top < (VIEW_TOP * d) as i32 && px - r.left > ((CONTENT_X + 70.0) * d) as i32
            })
            .unwrap_or(false);
            LRESULT(if drag { HTCAPTION } else { HTCLIENT } as isize)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) as i16) as f32;
            with_win(|w| {
                let d = w.dpi;
                let (screen_x, _) = xy(lp);
                if screen_x - (w.popup.rect.0 as f32) < SIDEBAR_W * d {
                    let max = w.side_view.1;
                    let next = (w.side_scroll - delta / 120.0 * 40.0 * d).clamp(0.0, max);
                    if next != w.side_scroll {
                        w.side_scroll = next;
                        w.render();
                    }
                    return;
                }
                w.scroll_content(w.scroll - delta / 120.0 * 48.0 * d);
            });
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = xy(lp);
            let grabbed = with_win(|w| {
                let Some((r, th)) = w.side_bar else {
                    return false;
                };
                if !(x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3) {
                    return false;
                }
                let max = w.side_view.1;
                let ty = r.1 + (r.3 - th) * (w.side_scroll / max.max(0.001));
                let grab = if y >= ty && y < ty + th {
                    y - ty
                } else {
                    th / 2.0
                };
                w.side_drag = Some(grab);
                w.side_scroll = (((y - grab - r.1) / (r.3 - th).max(1.0)) * max).clamp(0.0, max);
                w.render();
                true
            })
            .unwrap_or(false);
            if grabbed {
                SetCapture(hwnd);
                return LRESULT(0);
            }
            let started = with_win(|w| match w.hit(x, y) {
                Some(Action::Slider(id, sx, sw)) => {
                    w.dragging = Some((id, sx, sw));
                    w.slide(id, x, sx, sw, false);
                    w.render();
                    true
                }
                Some(_) => {
                    w.press = Some((x, y));
                    w.render();
                    false
                }
                None => false,
            })
            .unwrap_or(false);
            if started {
                let _ = SetCapture(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = xy(lp);
            with_win(|w| {
                if let (Some(grab), Some((r, th))) = (w.side_drag, w.side_bar) {
                    let max = w.side_view.1;
                    w.side_scroll =
                        (((y - grab - r.1) / (r.3 - th).max(1.0)) * max).clamp(0.0, max);
                    w.render();
                    return;
                }
                if let Some((id, sx, sw)) = w.dragging {
                    w.slide(id, x, sx, sw, false);
                    return;
                }
                let mut t = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut t);
                let (top, s) = (w.side_view.0, w.side_scroll);
                let hit = if y < top || y > SIDE_BOTTOM * w.dpi {
                    None
                } else {
                    w.side
                        .iter()
                        .find(|(_, r, _)| {
                            x >= r.0 && x < r.0 + r.2 && y + s >= r.1 && y + s < r.1 + r.3
                        })
                        .map(|e| e.0)
                };
                w.hover_side(hit);
            });
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            with_win(|w| w.hover_side(None));
            LRESULT(0)
        }
        WM_CHAR => {
            let ch = wp.0 as u32;
            with_win(|w| {
                if w.edit.is_some() {
                    if ch >= 32 && ch != 127 && w.edit_buf.chars().count() < 60 {
                        if let Some(c) = char::from_u32(ch) {
                            w.edit_buf.push(c);
                            w.render();
                        }
                    }
                    return;
                }
                if w.search_focus && ch >= 32 && ch != 127 && w.search.chars().count() < 80 {
                    if let Some(c) = char::from_u32(ch) {
                        w.search.push(c);
                        w.render();
                    }
                }
            });
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = wp.0 as u16;
            with_win(|w| {
                if w.edit.is_some() {
                    match vk {
                        0x08 => {
                            w.edit_buf.pop();
                            w.render();
                        }
                        0x1B => {
                            w.edit = None;
                            w.render();
                        }
                        0x0D => w.commit_edit(),
                        _ => {}
                    }
                    return;
                }
                if vk == b'F' as u16 && GetKeyState(VK_CONTROL.0 as i32) < 0 {
                    w.search_focus = true;
                    w.render();
                    return;
                }
                if !w.search_focus {
                    let page = (H - VIEW_TOP - 42.0) * w.dpi;
                    let offset = match vk {
                        0x21 => w.scroll - page, // Page Up
                        0x22 => w.scroll + page, // Page Down
                        0x24 => 0.0,             // Home
                        0x23 => w.content_h,     // End
                        _ => return,
                    };
                    w.scroll_content(offset);
                    return;
                }
                match vk {
                    0x08 => {
                        w.search.pop();
                    }
                    0x1B => {
                        w.search.clear();
                        w.search_focus = false;
                    }
                    0x0D => {
                        if !w.search.trim().is_empty() {
                            if let Some((page, _, _)) = w.side.first() {
                                w.select_page(*page);
                                return;
                            }
                        }
                    }
                    _ => return,
                }
                w.render();
            });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = xy(lp);
            let _ = ReleaseCapture();
            if with_win(|w| w.side_drag.take().is_some()).unwrap_or(false) {
                with_win(|w| w.render());
                return LRESULT(0);
            }
            let act = with_win(|w| {
                let was_pressed = w.press.take().is_some();
                if let Some((id, sx, sw)) = w.dragging.take() {
                    w.slide(id, x, sx, sw, true);
                    w.render();
                    return None;
                }
                if was_pressed {
                    w.render();
                }
                w.hit(x, y)
            })
            .flatten();
            if !matches!(
                act,
                Some(Action::Search | Action::ClearSearch | Action::Tab(_))
            ) {
                with_win(|w| {
                    if w.search_focus {
                        w.search_focus = false;
                        w.render();
                    }
                });
            }
            if !matches!(act, Some(Action::EditText(_))) {
                with_win(|w| {
                    if w.edit.is_some() {
                        w.commit_edit();
                    }
                });
            }
            match act {
                Some(Action::EditText(f)) => {
                    with_win(|w| {
                        if w.edit != Some(f) {
                            if w.edit.is_some() {
                                w.commit_edit();
                            }
                            let s = prefs::get();
                            w.edit_buf = if f == 0 {
                                s.greet_name.as_str().to_string()
                            } else {
                                s.greet_text.as_str().to_string()
                            };
                            w.edit = Some(f);
                            w.render();
                        }
                    });
                    let _ = SetForegroundWindow(hwnd);
                    let _ = SetFocus(Some(hwnd));
                }
                Some(Action::NotchIdle(v)) => prefs::update(|s| s.notch_idle = v),
                Some(Action::LyricsCompact) => {
                    prefs::update(|s| s.lyrics_compact = !s.lyrics_compact)
                }
                Some(Action::NotchDownloads) => {
                    prefs::update(|s| s.notch_downloads = !s.notch_downloads)
                }
                Some(Action::NotchSend) => prefs::update(|s| s.notch_send = !s.notch_send),
                Some(Action::LyricsOnline) => prefs::update(|s| s.lyrics_online = !s.lyrics_online),

                Some(Action::MenuLogoPick) => choose_logo(hwnd, false),
                Some(Action::LaunchpadKey(key)) => prefs::update(|s| s.launchpad_key = key),
                Some(Action::LaunchpadIcon(icon)) => {
                    if icon == 4 || (icon == 3 && crate::menulogo::launcher_path().is_none()) {
                        choose_logo(hwnd, true);
                    } else {
                        prefs::update(|s| s.launchpad_icon = icon);
                    }
                }
                Some(Action::MenuLogoMode(mode)) => {
                    if mode == 2 && crate::menulogo::path().is_none() {
                        choose_logo(hwnd, false);
                    } else {
                        prefs::update(|s| s.menu_logo = mode.min(2));
                    }
                }
                Some(Action::UiScale(v)) => prefs::update(|s| s.ui_scale = v),
                Some(Action::WidgetStyle(v)) => prefs::update(|s| s.widget_style = v),
                Some(Action::BatteryPct(v)) => prefs::update(|s| s.battery_pct = v),
                Some(Action::WidgetGallery) => crate::widgets::open_gallery(),
                Some(Action::NotchShape(b)) => prefs::update(|s| s.notch_island = b),
                Some(Action::MenuLogoReset) => {
                    crate::menulogo::reset();
                    prefs::update(|s| s.menu_logo = 1);
                    prefs::broadcast();
                }
                Some(Action::LyricsDir) => {
                    let dir = crate::paths::config_dir().join("lyrics");
                    let _ = std::fs::create_dir_all(&dir);
                    azr_platform::launch::open(&dir.to_string_lossy());
                }
                Some(Action::ClearSearch) => {
                    with_win(|w| {
                        w.search.clear();
                        w.search_focus = true;
                        w.side_scroll = 0.0;
                        w.render();
                    });
                    let _ = SetForegroundWindow(hwnd);
                    let _ = SetFocus(Some(hwnd));
                }
                Some(Action::Search) => {
                    with_win(|w| {
                        w.search_focus = true;
                        w.render();
                    });
                    let _ = SetForegroundWindow(hwnd);
                    let _ = SetFocus(Some(hwnd));
                }
                Some(Action::Back) => {
                    with_win(|w| {
                        if w.hpos > 0 {
                            w.hpos -= 1;
                            let t = w.history[w.hpos];
                            w.go(t, false);
                        }
                    });
                }
                Some(Action::Forward) => {
                    with_win(|w| {
                        if w.hpos + 1 < w.history.len() {
                            w.hpos += 1;
                            let t = w.history[w.hpos];
                            w.go(t, false);
                        }
                    });
                }
                Some(Action::IconVariant(v)) => prefs::update(|s| s.icon_variant = v),
                Some(Action::MinEffect(v)) => prefs::update(|s| s.min_effect = v),
                Some(Action::MainDisplay(id)) => prefs::update(|s| s.main_display = id),
                Some(Action::ProjectRepo) => {
                    azr_platform::launch::open("https://github.com/Ericismee/Azr")
                }
                Some(Action::ProjectSupport) => {
                    azr_platform::launch::open("https://ericismee.github.io/Azr/")
                }
                Some(Action::Minimize) => {
                    let _ = ShowWindow(hwnd, SW_MINIMIZE);
                }
                Some(Action::Close) => {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE_SELF, WPARAM(0), LPARAM(0));
                }
                Some(Action::Takeover) => prefs::update(|s| s.takeover = !s.takeover),
                Some(Action::Dock(b)) => prefs::update(|s| s.dock = b),
                Some(Action::Notch(b)) => prefs::update(|s| s.notch = b),
                Some(Action::Theme(t)) => prefs::update(|s| s.theme = t),
                Some(Action::Lang(l)) => prefs::update(|s| s.lang = l),
                Some(Action::WinShadow) => prefs::update(|s| s.win_shadow = !s.win_shadow),
                Some(Action::HoverOpen) => {
                    prefs::update(|s| s.notch_hover_open = !s.notch_hover_open)
                }
                Some(Action::MenuBarOn) => prefs::update(|s| s.menubar = !s.menubar),
                Some(Action::HotCycle(i)) => prefs::update(|s| s.hot[i] = s.hot[i].next()),
                Some(Action::StageOn) => prefs::update(|s| s.stage = !s.stage),
                Some(Action::SwitcherOn) => {
                    prefs::update(|s| s.overview_alt_tab = !s.overview_alt_tab)
                }
                Some(Action::StageFlag(k)) => prefs::update(|s| match k {
                    1 => s.stage_desktop_click = !s.stage_desktop_click,
                    2 => s.stage_media = !s.stage_media,
                    _ => s.stage_titles = !s.stage_titles,
                }),
                Some(Action::StageRight(r)) => prefs::update(|s| s.stage_right = r),
                Some(Action::StageIcons(m)) => prefs::update(|s| s.stage_icons = m),
                Some(Action::MacKeys) => prefs::update(|s| s.mac_shortcuts = !s.mac_shortcuts),
                Some(Action::Accent(i)) => prefs::update(|s| s.accent = i),
                Some(Action::IconStyle(m)) => prefs::update(|s| {
                    s.icon_style = match m {
                        0 => azr_core::settings::IconStyle::Default,
                        1 => azr_core::settings::IconStyle::Dark,
                        2 => azr_core::settings::IconStyle::Clear,
                        _ => azr_core::settings::IconStyle::Tinted,
                    }
                }),
                Some(Action::IconTint(i)) => prefs::update(|s| s.icon_tint = i),
                Some(Action::ClipHistory) => {
                    prefs::update(|s| s.clipboard_history = !s.clipboard_history)
                }
                Some(Action::MacIconCorners) => prefs::update(|s| {
                    s.icon_round = 75;
                    s.dock_plates = true;
                }),
                Some(Action::DockSurface(v)) => prefs::update(|s| s.dock_surface = v.min(3)),
                Some(Action::IconBackground(v)) => prefs::update(|s| s.icon_background = v.min(2)),
                Some(Action::DockPlates) => prefs::update(|s| s.dock_plates = !s.dock_plates),
                Some(Action::IconPack) => prefs::update(|s| s.icon_pack = !s.icon_pack),
                Some(Action::DownloadsStack) => {
                    prefs::update(|s| s.downloads_stack = !s.downloads_stack)
                }
                Some(Action::RecentApps) => prefs::update(|s| s.recent_apps = !s.recent_apps),
                Some(Action::HideIcons) => {
                    prefs::update(|s| s.hide_desktop_icons = !s.hide_desktop_icons)
                }
                Some(Action::BarStyle(m)) => prefs::update(|s| match m {
                    0 => {
                        s.menubar_glass = false;
                        s.menubar_material = false;
                    }
                    3 => {
                        s.menubar_glass = false;
                        s.menubar_material = true;
                    }
                    1 => {
                        s.menubar_material = false;
                        s.menubar_glass = true;
                        s.menubar_style = azr_core::settings::BarStyle::Light;
                    }
                    _ => {
                        s.menubar_material = false;
                        s.menubar_glass = true;
                        s.menubar_style = azr_core::settings::BarStyle::Dark;
                    }
                }),
                Some(Action::Clock24) => prefs::update(|s| s.clock_24h = !s.clock_24h),
                Some(Action::DockOn) => prefs::update(|s| s.dock_enabled = !s.dock_enabled),
                Some(Action::NotchOn) => prefs::update(|s| s.notch_enabled = !s.notch_enabled),
                Some(Action::Refraction) => prefs::update(|s| s.refraction = !s.refraction),
                Some(Action::Anim) => prefs::update(|s| s.anim = !s.anim),
                Some(Action::Autostart) => prefs::update(|s| s.autostart = !s.autostart),
                Some(Action::Material(true, m)) => prefs::update(|s| s.dock_material = m),
                Some(Action::Material(false, m)) => prefs::update(|s| s.notch_material = m),
                Some(Action::Tab(t)) => {
                    with_win(|w| w.select_page(t));
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_PREFS => {
            with_win(|w| w.render());
            let (themed, _) = (prefs::look(Role::Themed), ());
            with_win(|w| {
                let base = azr_ui::glass::GlassParams::dark((10.0, 10.0), 24.0 * w.dpi);
                let _ = w
                    .popup
                    .panel
                    .set_params(side_glass(prefs::apply(base, &themed)));
            });
            LRESULT(0)
        }
        WM_CLOSE_SELF => {
            prefs::unlisten(hwnd);
            let win = WIN.with(|s| s.borrow_mut().take());
            drop(win);
            let _ = DestroyWindow(hwnd);
            azr_platform::sys::trim_later(5000);
            log::line("settings", "ditutup");
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn choose_logo(hwnd: HWND, launchpad: bool) {
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_NOTOPMOST),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
    let picked = azr_platform::pick::open_file(
        prefs::tr("Choose image"),
        &[(
            prefs::tr("Logo image"),
            "*.png;*.jpg;*.jpeg;*.bmp;*.ico;*.gif;*.webp;*.tif;*.tiff",
        )],
    );
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
    if let Some(p) = picked {
        let saved = if launchpad {
            crate::menulogo::set_launcher(&p)
        } else {
            crate::menulogo::set(&p)
        };
        if saved {
            prefs::update(|s| {
                if launchpad {
                    s.launchpad_icon = 3;
                } else {
                    s.menu_logo = 2;
                }
            });
            prefs::broadcast();
        } else {
            let text = windows::core::HSTRING::from(prefs::tr(
                "Choose a supported image up to 8 MB. The previous logo was kept.",
            ));
            let _ = MessageBoxW(Some(hwnd), &text, w!("Azr"), MB_OK | MB_ICONWARNING);
        }
    }
}

#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn settings_search_accepts_unaccented_vietnamese_and_out_of_order_words() {
        assert!(search_matches("Ẩn thanh tác vụ Windows", "windows tac vu"));
        assert!(search_matches("Chuyển giữa các cửa sổ", "cua so"));
        assert!(search_matches("Điều khiển", "dieu"));
        assert!(search_matches("Ta\u{0301}c vu\u{0323}", "tac vu"));
        assert!(!search_matches("Ẩn thanh tác vụ Windows", "tac vu macos"));
    }
}
