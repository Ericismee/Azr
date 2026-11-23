use crate::popup::Popup;
use azr_platform::text::{render, Style};
use azr_platform::wins::thumbnail;
use azr_ui::gfx::Gfx;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
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

struct Cell {
    left: i32,
    right: i32,
    top: i32,
    hwnd: isize,
    close: (i32, i32, i32),
    thumb: Option<isize>,
}

pub struct Preview {
    popup: Popup,
    cells: Vec<Cell>,
    dpi: f32,
    bounds: (i32, i32, i32, i32),
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
        self.clear();
        let d = self.dpi;
        let n = windows.len();
        if n == 0 {
            return Ok(());
        }
        let (pad, gap, title_h) = ((12.0 * d) as i32, (6.0 * d) as i32, (26.0 * d) as i32);
        let (left, top, right, _) = self.bounds;
        let (cols, rows, tw, th) = preview_grid(n, right - left, (bottom - top).max(1), d);
        let w = pad + cols as i32 * (tw + pad);
        let stride = th + gap + title_h + pad;
        let h = pad + rows as i32 * stride;
        let x = (cx - w / 2).clamp(left + 4, (right - w - 4).max(left + 4));
        self.popup.set_rect(x, (bottom - h).max(top), w, h)?;
        self.popup.panel.set_radius(18.0 * d)?;
        let style = Style {
            px: (12.0 * d).round() as i32,
            weight: 500,
        };
        for (i, (hwnd, title, minimized)) in windows.iter().enumerate() {
            let x0 = pad + (i % cols) as i32 * (tw + pad);
            let row_y = pad + (i / cols) as i32 * stride;
            let thumb = if *minimized {
                None
            } else {
                thumbnail::register(self.popup.hwnd.0 as isize, *hwnd)
            };
            if *minimized {
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
            if let Some(t) = thumb {
                let (sw, sh) = thumbnail::source_size(t)
                    .filter(|s| s.0 > 0 && s.1 > 0)
                    .unwrap_or((tw, th));
                let k = (tw as f32 / sw as f32).min(th as f32 / sh as f32).min(1.0);
                let (dw, dh) = (
                    ((sw as f32 * k) as i32).max(1),
                    ((sh as f32 * k) as i32).max(1),
                );
                let (dx, dy) = (x0 + (tw - dw) / 2, row_y + (th - dh) / 2);
                thumbnail::place(t, (dx, dy, dx + dw, dy + dh), 255);
            }
            let close_size = title_h.min(tw);
            let title_y = row_y + th + gap;
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
            let close_x = x0 + tw - close_size;
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
            self.cells.push(Cell {
                left: x0 - pad / 2,
                right: x0 + tw + pad / 2,
                top: row_y,
                hwnd: *hwnd,
                close: (close_x, title_y, close_size),
                thumb,
            });
        }
        self.popup.show();
        Ok(())
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        self.cells.iter().find_map(|cell| cell.hit(x, y))
    }

    fn clear(&mut self) {
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
        if self.popup.is_visible() || !self.cells.is_empty() {
            self.clear();
            self.popup.hide();
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
