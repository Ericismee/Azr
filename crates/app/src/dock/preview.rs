use crate::popup::Popup;
use azr_platform::text::{render, Style};
use azr_platform::wins::{snapshot, thumbnail};
use azr_ui::gfx::Gfx;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{MonitorFromRect, MONITOR_DEFAULTTONULL};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::UI::Composition::{CompositionDrawingSurface, CompositionStretch, Compositor};
use windows_core::Result;
use windows_numerics::{Vector2, Vector3};

pub const PREVIEW_CLICK: u32 = WM_APP + 7;
pub const PREVIEW_ENTER: u32 = WM_APP + 8;
pub const PREVIEW_LEAVE: u32 = WM_APP + 9;
pub const PREVIEW_MOVE: u32 = WM_APP + 16;
const WM_MOUSELEAVE: u32 = 0x02A3;

static TARGET: AtomicIsize = AtomicIsize::new(0);
static INSIDE: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let target = HWND(TARGET.load(Ordering::Relaxed) as *mut _);
    match m {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            if !INSIDE.swap(true, Ordering::Relaxed) {
                let mut t = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: h,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut t);
                let _ = PostMessageW(Some(target), PREVIEW_ENTER, WPARAM(0), LPARAM(0));
            }
            let _ = SendMessageW(target, PREVIEW_MOVE, Some(WPARAM(0)), Some(l));
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            INSIDE.store(false, Ordering::Relaxed);
            let _ = PostMessageW(Some(target), PREVIEW_LEAVE, WPARAM(0), LPARAM(0));
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            SetCapture(h);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if GetCapture() == h {
                let _ = ReleaseCapture();
                let _ = SendMessageW(target, PREVIEW_CLICK, Some(WPARAM(0)), Some(l));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(h, m, w, l),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Activate(isize),
    Close(isize),
}

#[derive(Default)]
struct RetainedThumbnails(HashMap<isize, isize>);

impl Drop for RetainedThumbnails {
    fn drop(&mut self) {
        for handle in self.0.values().copied() {
            thumbnail::unregister(handle);
        }
    }
}

struct Cell {
    left: i32,
    right: i32,
    top: i32,
    hwnd: isize,
    close: (i32, i32, i32),
    thumb: Option<isize>,
}

struct Peek {
    popup: Popup,
    thumbnail: Option<isize>,
}

unsafe extern "system" fn peek_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        _ => DefWindowProcW(h, m, w, l),
    }
}

impl Peek {
    fn new(comp: &Compositor) -> Result<Self> {
        let popup = Popup::new(
            comp,
            w!("AzrDockPeek"),
            Some(peek_proc),
            WS_EX_TRANSPARENT,
            crate::prefs::glass((10.0, 10.0), 0.0, crate::prefs::Role::Dark),
        )?;
        Ok(Self {
            popup,
            thumbnail: None,
        })
    }

    fn show(&mut self, source: isize, previews: HWND) -> Result<()> {
        self.hide();
        let window = HWND(source as *mut _);
        // A minimized source may not have a composited frame. Keep its small preview.
        if unsafe { !IsWindowVisible(window).as_bool() || IsIconic(window).as_bool() } {
            return Ok(());
        }
        let Some((x, y, w, h)) = snapshot::frame_rect(source) else {
            return Ok(());
        };
        if w <= 0 || h <= 0 {
            return Ok(());
        }
        let rect = windows::Win32::Foundation::RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        if unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONULL).is_invalid() } {
            return Ok(());
        }
        self.popup.set_rect(x, y, w, h)?;
        let Some(handle) = thumbnail::register(self.popup.hwnd.0 as isize, source) else {
            return Ok(());
        };
        self.thumbnail = Some(handle);
        if !thumbnail::place(handle, (0, 0, w, h), 255) {
            self.hide();
            return Ok(());
        }
        unsafe {
            SetWindowPos(
                self.popup.hwnd,
                Some(previews),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
        }
    }

    fn hide(&mut self) {
        self.popup.hide();
        if let Some(handle) = self.thumbnail.take() {
            thumbnail::unregister(handle);
        }
    }
}

impl Drop for Peek {
    fn drop(&mut self) {
        self.hide();
        unsafe {
            let _ = DestroyWindow(self.popup.hwnd);
        }
    }
}

pub struct Preview {
    popup: Popup,
    cells: Vec<Cell>,
    dpi: f32,
    bounds: (i32, i32, i32, i32),
    windows: Vec<(isize, String, bool)>,
    peek: Option<Peek>,
    pending_peek: Option<isize>,
}

impl Preview {
    pub fn new(
        comp: &Compositor,
        dock_hwnd: HWND,
        dpi: f32,
        bounds: (i32, i32, i32, i32),
    ) -> Result<Preview> {
        TARGET.store(dock_hwnd.0 as isize, Ordering::Relaxed);
        let popup = Popup::new(
            comp,
            w!("AzrPreview"),
            Some(proc),
            WINDOW_EX_STYLE(0),
            crate::prefs::glass((10.0, 10.0), 18.0 * dpi, crate::prefs::Role::Dark),
        )?;
        Ok(Preview {
            popup,
            cells: vec![],
            dpi,
            bounds,
            windows: Vec::new(),
            peek: None,
            pending_peek: None,
        })
    }

    pub fn panel_mut(&mut self) -> &mut azr_ui::glass::GlassPanel {
        &mut self.popup.panel
    }

    pub fn is_visible(&self) -> bool {
        self.popup.is_visible()
    }

    pub fn show(
        &mut self,
        comp: &Compositor,
        gfx: &Gfx,
        windows: &[(isize, String, bool)],
        icon: Option<&CompositionDrawingSurface>,
        cx: i32,
        bottom: i32,
    ) -> Result<()> {
        let d = self.dpi;
        let n = windows.len();
        if n == 0 {
            self.hide();
            return Ok(());
        }
        let (pad, gap, title_h) = ((12.0 * d) as i32, (6.0 * d) as i32, (26.0 * d) as i32);
        let (left, top, right, _) = self.bounds;
        let (cols, rows, tw, th) = preview_grid(n, right - left, (bottom - top).max(1), d);
        let w = pad + cols as i32 * (tw + pad);
        let stride = th + gap + title_h + pad;
        let h = pad + rows as i32 * stride;
        let x = (cx - w / 2).clamp(left + 4, (right - w - 4).max(left + 4));
        let rect = (x, (bottom - h).max(top), w, h);
        let (_, _, previous_width, previous_height) = self.popup.rect;
        if self.popup.rect != rect {
            self.popup.set_rect(rect.0, rect.1, rect.2, rect.3)?;
        }
        if self.is_visible()
            && self.windows == windows
            && previous_width == w
            && previous_height == h
        {
            return Ok(());
        }
        self.cancel_peek();
        self.windows.clear();
        let mut retained = RetainedThumbnails::default();
        for cell in self.cells.drain(..) {
            if let Some(handle) = cell.thumb {
                if windows.iter().any(|(hwnd, _, _)| *hwnd == cell.hwnd) {
                    if let Some(previous) = retained.0.insert(cell.hwnd, handle) {
                        thumbnail::unregister(previous);
                    }
                } else {
                    thumbnail::unregister(handle);
                }
            }
        }
        if let Ok(children) = self.popup.content.Children() {
            let _ = children.RemoveAll();
        }
        self.popup.panel.set_radius(18.0 * d)?;
        let style = Style {
            px: (12.0 * d).round() as i32,
            weight: 500,
        };
        for (i, (hwnd, title, minimized)) in windows.iter().enumerate() {
            let x0 = pad + (i % cols) as i32 * (tw + pad);
            let row_y = pad + (i / cols) as i32 * stride;
            let thumb = retained
                .0
                .remove(hwnd)
                .or_else(|| thumbnail::register(self.popup.hwnd.0 as isize, *hwnd))
                .and_then(|handle| match thumbnail::source_size(handle) {
                    Some((w, h)) if w > 0 && h > 0 => Some((handle, w, h)),
                    _ => {
                        thumbnail::unregister(handle);
                        None
                    }
                });
            let close_size = title_h.min(tw);
            let title_y = row_y + th + gap;
            let close_x = x0 + tw - close_size;
            self.cells.push(Cell {
                left: x0 - pad / 2,
                right: x0 + tw + pad / 2,
                top: row_y,
                hwnd: *hwnd,
                close: (close_x, title_y, close_size),
                thumb: thumb.map(|(handle, _, _)| handle),
            });
            if *minimized || thumb.is_none() {
                if let Some(s) = icon {
                    let sz = (64.0 * d) as i32;
                    let brush = comp.CreateSurfaceBrushWithSurface(s)?;
                    brush.SetStretch(CompositionStretch::Uniform)?;
                    let sprite = comp.CreateSpriteVisual()?;
                    sprite.SetBrush(&brush)?;
                    sprite.SetSize(Vector2 {
                        X: sz as f32,
                        Y: sz as f32,
                    })?;
                    sprite.SetOffset(Vector3 {
                        X: (x0 + (tw - sz) / 2) as f32,
                        Y: (row_y + (th - sz) / 2) as f32,
                        Z: 0.0,
                    })?;
                    sprite.SetOpacity(0.55)?;
                    self.popup.content.Children()?.InsertAtTop(&sprite)?;
                }
            }
            if let Some((t, sw, sh)) = thumb {
                let k = (tw as f32 / sw as f32).min(th as f32 / sh as f32).min(1.0);
                let (dw, dh) = (
                    ((sw as f32 * k) as i32).max(1),
                    ((sh as f32 * k) as i32).max(1),
                );
                let (dx, dy) = (x0 + (tw - dw) / 2, row_y + (th - dh) / 2);
                thumbnail::place(t, (dx, dy, dx + dw, dy + dh), 255);
            }
            let title_width = (tw - close_size - gap).max(1);
            if let Some(bmp) = render(title, &style, title_width, title_h) {
                let surface = gfx.surface_from_bgra(bmp.w, bmp.h, &bmp.bgra)?;
                let brush = comp.CreateSurfaceBrushWithSurface(&surface)?;
                brush.SetStretch(CompositionStretch::None)?;
                let sprite = comp.CreateSpriteVisual()?;
                sprite.SetBrush(&brush)?;
                sprite.SetSize(Vector2 {
                    X: title_width as f32,
                    Y: title_h as f32,
                })?;
                sprite.SetOffset(Vector3 {
                    X: x0 as f32,
                    Y: title_y as f32,
                    Z: 0.0,
                })?;
                self.popup.content.Children()?.InsertAtTop(&sprite)?;
            }
            let scale = close_size as f32;
            let center = scale / 2.0;
            let backdrop = comp.CreateSpriteVisual()?;
            let square = comp.CreateRoundedRectangleGeometry()?;
            square.SetSize(Vector2 { X: scale, Y: scale })?;
            square.SetCornerRadius(Vector2 {
                X: 7.0 * d,
                Y: 7.0 * d,
            })?;
            let clip = comp.CreateGeometricClip()?;
            clip.SetGeometry(&square)?;
            backdrop.SetClip(&clip)?;
            backdrop.SetBrush(&comp.CreateColorBrushWithColor(windows::UI::Color {
                A: 74,
                R: 228,
                G: 233,
                B: 244,
            })?)?;
            backdrop.SetSize(Vector2 { X: scale, Y: scale })?;
            backdrop.SetOffset(Vector3 {
                X: close_x as f32,
                Y: title_y as f32,
                Z: 0.0,
            })?;
            self.popup.content.Children()?.InsertAtTop(&backdrop)?;
            for angle in [-45.0, 45.0] {
                let stroke = comp.CreateSpriteVisual()?;
                stroke.SetSize(Vector2 {
                    X: 11.0 * d,
                    Y: 1.5 * d,
                })?;
                stroke.SetCenterPoint(Vector3 {
                    X: 5.5 * d,
                    Y: 0.75 * d,
                    Z: 0.0,
                })?;
                stroke.SetRotationAngleInDegrees(angle)?;
                stroke.SetBrush(&comp.CreateColorBrushWithColor(windows::UI::Color {
                    A: 255,
                    R: 255,
                    G: 255,
                    B: 255,
                })?)?;
                stroke.SetOffset(Vector3 {
                    X: close_x as f32 + center - 5.5 * d,
                    Y: title_y as f32 + center - 0.75 * d,
                    Z: 0.0,
                })?;
                self.popup.content.Children()?.InsertAtTop(&stroke)?;
            }
        }
        self.windows = windows.to_vec();
        self.popup.show();
        Ok(())
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        self.cells.iter().find_map(|cell| cell.hit(x, y))
    }

    pub fn update_peek_target(&mut self, x: i32, y: i32) -> Option<bool> {
        let target = self.cells.iter().find_map(|cell| cell.peek_target(x, y));
        if target == self.pending_peek {
            return None;
        }
        self.cancel_peek();
        self.pending_peek = target;
        Some(target.is_some())
    }

    pub fn show_peek(&mut self, comp: &Compositor) -> Result<()> {
        let Some(target) = self.pending_peek.filter(|_| self.is_visible()) else {
            return Ok(());
        };
        if self.peek.is_none() {
            self.peek = Some(Peek::new(comp)?);
        }
        if let Some(peek) = &mut self.peek {
            peek.show(target, self.popup.hwnd)?;
        }
        Ok(())
    }

    pub fn cancel_peek(&mut self) {
        self.pending_peek = None;
        if let Some(peek) = &mut self.peek {
            peek.hide();
        }
    }

    fn clear(&mut self) {
        self.windows.clear();
        for c in self.cells.drain(..) {
            if let Some(t) = c.thumb {
                thumbnail::unregister(t);
            }
        }
        if let Ok(kids) = self.popup.content.Children() {
            let _ = kids.RemoveAll();
        }
    }

    pub fn hide(&mut self) {
        self.cancel_peek();
        if self.popup.is_visible() || !self.cells.is_empty() {
            INSIDE.store(false, Ordering::Relaxed);
            self.clear();
            self.popup.hide();
        }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.hide();
        unsafe {
            let _ = DestroyWindow(self.popup.hwnd);
        }
    }
}

fn preview_grid(n: usize, width: i32, height: i32, dpi: f32) -> (usize, usize, i32, i32) {
    let pad = (12.0 * dpi) as i32;
    let title = (26.0 * dpi) as i32;
    let gap = (6.0 * dpi) as i32;
    let max_cols = ((width - 8 - pad) / (pad + (105.0 * dpi) as i32)).max(1) as usize;
    (1..=n.min(max_cols))
        .map(|cols| {
            let rows = n.div_ceil(cols);
            let w = ((width - 8 - pad) / cols as i32 - pad).clamp(1, (200.0 * dpi) as i32);
            let h = (((height - pad) / rows as i32) - gap - title - pad)
                .clamp(1, w.min((125.0 * dpi) as i32));
            (cols, rows, w, h)
        })
        .max_by_key(|&(cols, rows, w, h)| (w * h, std::cmp::Reverse(rows), std::cmp::Reverse(cols)))
        .unwrap_or((1, 1, 1, 1))
}

impl Cell {
    fn peek_target(&self, x: i32, y: i32) -> Option<isize> {
        match self.hit(x, y) {
            Some(Hit::Activate(hwnd)) => Some(hwnd),
            _ => None,
        }
    }

    fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        let (left, top, size) = self.close;
        if (left..left + size).contains(&x) && (top..top + size).contains(&y) {
            Some(Hit::Close(self.hwnd))
        } else if (self.left..self.right).contains(&x) && (self.top..top + size).contains(&y) {
            Some(Hit::Activate(self.hwnd))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_targets_the_window_body_and_ignores_close_and_card_gaps() {
        let cell = Cell {
            left: 8,
            right: 220,
            top: 12,
            hwnd: 17,
            close: (180, 145, 26),
            thumb: None,
        };
        assert_eq!(cell.peek_target(54, 120), Some(17));
        assert_eq!(cell.peek_target(54, 150), Some(17));
        assert_eq!(cell.peek_target(191, 150), None);
        assert_eq!(cell.peek_target(54, 172), None);
    }

    #[test]
    fn wraps_a_large_group_inside_a_narrow_monitor() {
        let (cols, rows, w, h) = preview_grid(9, 780, 700, 1.5);
        assert!(rows > 1 && cols * rows >= 9);
        assert!(18 + cols as i32 * (w + 18) <= 772);
        assert!(18 + rows as i32 * (h + 9 + 39 + 18) <= 700);
    }

    #[test]
    fn close_button_does_not_overlap_the_activation_area() {
        let window = Cell {
            left: 8,
            right: 220,
            top: 12,
            hwnd: 17,
            close: (180, 145, 26),
            thumb: None,
        };
        assert_eq!(window.hit(191, 153), Some(Hit::Close(17)));
        assert_eq!(window.hit(191, 130), Some(Hit::Activate(17)));
        assert_eq!(window.hit(177, 153), Some(Hit::Activate(17)));
        assert_eq!(window.hit(191, 173), None);
    }
}
