//! Ventana hija Win32 sobre la que dibuja mpv, colocada encima del hueco que reserva la interfaz.

use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use super::state::lock;
use windows::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BLACK_BRUSH, ClientToScreen, CombineRgn, CreateRectRgn, DeleteObject, GetRegionData,
    GetStockObject, GetWindowRgn, HBRUSH, HRGN, MapWindowPoints, RGN_DIFF, RGNDATAHEADER,
    SetWindowRgn,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, EnumChildWindows, EnumWindows, GA_ROOT, GW_CHILD, GW_HWNDNEXT,
    GWL_STYLE, GetAncestor, GetClassNameW, GetParent, GetWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, HWND_TOP, IsChild, IsWindowVisible, RegisterClassW,
    SW_HIDE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WM_CREATE,
    WM_MOUSEHWHEEL, WM_MOUSEWHEEL, WM_PARENTNOTIFY, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN,
    WS_CLIPSIBLINGS, WindowFromPoint,
};
use windows::core::{BOOL, PCWSTR, w};

const CLASS_NAME: PCWSTR = w!("MediaCenterVideoSurface");
const MAX_SIDE: i32 = 16_384;

/// Rectángulo en píxeles físicos, relativo al área cliente de la ventana principal.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl SurfaceRect {
    pub fn validate(self) -> Result<Self, String> {
        let within = |side: i32| (1..=MAX_SIDE).contains(&side);
        let origin = |value: i32| (-MAX_SIDE..=MAX_SIDE).contains(&value);
        if within(self.width) && within(self.height) && origin(self.x) && origin(self.y) {
            Ok(self)
        } else {
            Err(format!("Rectángulo de superficie no válido: {self:?}"))
        }
    }

    fn from_rect(rect: RECT) -> Self {
        Self {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }
}

/// Posición de la superficie: el hueco completo y la parte de él que la interfaz deja ver.
/// La ventana nativa no queda recortada por el `overflow` del HTML, así que el recorte lo hace la
/// región de la ventana.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SurfacePlacement {
    pub bounds: SurfaceRect,
    pub visible: SurfaceRect,
}

impl SurfacePlacement {
    pub fn validate(self) -> Result<Self, String> {
        Ok(Self {
            bounds: self.bounds.validate()?,
            visible: self.visible.validate()?,
        })
    }

    /// Parte visible relativa a la superficie, o `None` si se ve entera.
    fn visible_part(&self) -> Option<SurfaceRect> {
        let part = self.relative_visible();
        (part != self.whole()).then_some(part)
    }

    fn whole(&self) -> SurfaceRect {
        SurfaceRect {
            x: 0,
            y: 0,
            width: self.bounds.width,
            height: self.bounds.height,
        }
    }

    fn relative_visible(&self) -> SurfaceRect {
        let outer = self.bounds;
        let inner = SurfaceRect {
            x: self.visible.x - outer.x,
            y: self.visible.y - outer.y,
            ..self.visible
        };
        inner.intersect(self.whole())
    }

    /// Región mientras la ventana pasa de `previous` a `self`: lo que se ve en las dos posiciones.
    /// Con la región nueva y la posición antigua, el vídeo asomaría fuera de la zona visible.
    fn moving_part(&self, previous: &SurfacePlacement) -> SurfaceRect {
        self.relative_visible()
            .intersect(previous.relative_visible())
    }
}

impl SurfaceRect {
    fn intersect(self, other: SurfaceRect) -> SurfaceRect {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        SurfaceRect {
            x: left,
            y: top,
            width: (right - left).max(0),
            height: (bottom - top).max(0),
        }
    }
}

/// Evento con el giro de la rueda recibido sobre el vídeo, para desplazar la interfaz.
pub const WHEEL_EVENT: &str = "engine-surface-wheel";

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SurfaceWheel {
    pub delta: i16,
    pub horizontal: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ClientPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub handle: String,
    pub class_name: String,
    pub title: String,
    pub process_id: u32,
    pub visible: bool,
    pub enabled: bool,
    pub inside_main_window: bool,
    pub rect: SurfaceRect,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitTest {
    pub point: ClientPoint,
    pub class_name: String,
    pub hits_video: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowReport {
    pub surface_visible: bool,
    pub surface_rect: SurfaceRect,
    /// Ventanas visibles de nivel superior de Media Center o de mpv, sin contar la principal.
    pub stray_windows: Vec<WindowInfo>,
    pub video_windows: Vec<WindowInfo>,
    pub region: Vec<SurfaceRect>,
    pub hit_tests: Vec<HitTest>,
}

pub struct VideoSurface {
    hwnd: isize,
    parent: isize,
    /// Última posición aplicada mientras la superficie está visible.
    placed: Mutex<Option<SurfacePlacement>>,
}

impl VideoSurface {
    pub fn create(app: &AppHandle) -> Result<Self, String> {
        let window = app
            .get_webview_window("main")
            .ok_or("No se encuentra la ventana principal")?;
        let parent = window.hwnd().map_err(|error| error.to_string())?.0 as isize;
        let _ = WHEEL_TARGET.set(app.clone());
        let hwnd = on_main_thread(app, move || unsafe { create_window(parent) })??;
        Ok(Self {
            hwnd,
            parent,
            placed: Mutex::new(None),
        })
    }

    /// Identificador para la opción `wid` de mpv: un HWND cabe en 32 bits y mpv lo espera así.
    pub fn wid(&self) -> i64 {
        i64::from(self.hwnd as u32)
    }

    /// Coloca la superficie y recorta en ella lo que no se ve y los huecos, que dejan ver el HTML
    /// de debajo y le pasan los eventos. Todo en una sola llamada al hilo principal.
    pub fn place(
        &self,
        app: &AppHandle,
        placement: Option<SurfacePlacement>,
        holes: Vec<SurfaceRect>,
    ) -> Result<(), String> {
        let hwnd = self.hwnd;
        let mut placed = lock(&self.placed);
        let previous = *placed;
        on_main_thread(app, move || unsafe {
            place_window(hwnd, previous, placement, &holes)
        })??;
        *placed = placement;
        Ok(())
    }

    pub fn report(
        &self,
        app: &AppHandle,
        mpv_process: Option<u32>,
        points: Vec<ClientPoint>,
    ) -> Result<WindowReport, String> {
        let (hwnd, parent) = (self.hwnd, self.parent);
        on_main_thread(app, move || unsafe {
            build_report(hwnd, parent, mpv_process, &points)
        })
    }
}

fn on_main_thread<T: Send + 'static>(
    app: &AppHandle,
    task: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(task());
    })
    .map_err(|error| error.to_string())?;
    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "El hilo principal de la ventana no respondió".to_string())
}

fn handle(value: isize) -> HWND {
    HWND(value as *mut _)
}

static WHEEL_TARGET: OnceLock<AppHandle> = OnceLock::new();

unsafe extern "system" fn surface_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        // mpv crea su ventana de vídeo dentro de la superficie. Deshabilitada, el ratón llega a la
        // superficie en lugar de a mpv, que lo ignoraría.
        WM_PARENTNOTIFY if (wparam.0 & 0xFFFF) as u32 == WM_CREATE => {
            unsafe {
                let _ = EnableWindow(HWND(lparam.0 as *mut _), false);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            if let Some(app) = WHEEL_TARGET.get() {
                let wheel = SurfaceWheel {
                    delta: ((wparam.0 >> 16) & 0xFFFF) as u16 as i16,
                    horizontal: message == WM_MOUSEHWHEEL,
                };
                let _ = app.emit(WHEEL_EVENT, wheel);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe fn register_class() -> Result<(), String> {
    static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| unsafe {
            let instance = GetModuleHandleW(None).map_err(|error| error.to_string())?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(surface_proc),
                hInstance: instance.into(),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
                return Err(format!(
                    "No se pudo registrar la superficie de vídeo: {:?}",
                    GetLastError()
                ));
            }
            Ok(())
        })
        .clone()
}

unsafe fn create_window(parent: isize) -> Result<isize, String> {
    unsafe {
        register_class()?;
        let instance = GetModuleHandleW(None).map_err(|error| error.to_string())?;
        let surface = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS_NAME,
            w!(""),
            WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
            0,
            0,
            0,
            0,
            Some(handle(parent)),
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|error| format!("No se pudo crear la superficie de vídeo: {error}"))?;
        clip_siblings(handle(parent), surface);
        Ok(surface.0 as isize)
    }
}

/// Sin `WS_CLIPSIBLINGS` en todas las hermanas, WebView2 puede pintar encima de la superficie.
unsafe fn clip_siblings(parent: HWND, surface: HWND) {
    unsafe {
        let mut child = GetWindow(parent, GW_CHILD).ok();
        while let Some(sibling) = child {
            if sibling != surface {
                let style = GetWindowLongPtrW(sibling, GWL_STYLE);
                SetWindowLongPtrW(sibling, GWL_STYLE, style | WS_CLIPSIBLINGS.0 as isize);
                let _ = SetWindowPos(
                    sibling,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
            child = GetWindow(sibling, GW_HWNDNEXT).ok();
        }
    }
}

unsafe fn place_window(
    surface: isize,
    previous: Option<SurfacePlacement>,
    placement: Option<SurfacePlacement>,
    holes: &[SurfaceRect],
) -> Result<(), String> {
    unsafe {
        let surface = handle(surface);
        match placement {
            Some(placement) => {
                // Ningún estado intermedio puede dejar ver vídeo fuera de la zona visible: si la
                // ventana se mueve, primero se recorta a lo que se ve en ambas posiciones.
                let moving = previous.filter(|previous| previous.bounds != placement.bounds);
                match moving {
                    Some(previous) => {
                        apply_region(surface, Some(placement.moving_part(&previous)), holes)
                    }
                    None => apply_region(surface, placement.visible_part(), holes),
                }
                let rect = placement.bounds;
                SetWindowPos(
                    surface,
                    Some(HWND_TOP),
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                )
                .map_err(|error| error.to_string())?;
                fit_children(surface, rect.width, rect.height);
                if moving.is_some() {
                    apply_region(surface, placement.visible_part(), holes);
                }
            }
            None => {
                let _ = ShowWindow(surface, SW_HIDE);
            }
        }
        Ok(())
    }
}

/// La ventana de vídeo de mpv es hija de la superficie y debe ocuparla entera. Se deshabilita
/// también aquí por si se creó antes de que la superficie pudiera recibir `WM_PARENTNOTIFY`.
unsafe fn fit_children(surface: HWND, width: i32, height: i32) {
    unsafe {
        for child in child_windows(surface) {
            if GetParent(child).ok() == Some(surface) {
                let _ = EnableWindow(child, false);
                let _ = SetWindowPos(
                    child,
                    None,
                    0,
                    0,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
}

unsafe fn apply_region(surface: HWND, visible: Option<SurfaceRect>, holes: &[SurfaceRect]) {
    unsafe {
        if visible.is_none() && holes.is_empty() {
            SetWindowRgn(surface, None, true);
            return;
        }
        let base = visible.unwrap_or(SurfaceRect {
            x: 0,
            y: 0,
            width: MAX_SIDE,
            height: MAX_SIDE,
        });
        let region = CreateRectRgn(base.x, base.y, base.x + base.width, base.y + base.height);
        for hole in holes {
            let cut = CreateRectRgn(hole.x, hole.y, hole.x + hole.width, hole.y + hole.height);
            CombineRgn(Some(region), Some(region), Some(cut), RGN_DIFF);
            let _ = DeleteObject(cut.into());
        }
        // Windows pasa a ser dueño de la región; no hay que liberarla después.
        SetWindowRgn(surface, Some(region), true);
    }
}

unsafe fn build_report(
    surface: isize,
    parent: isize,
    mpv_process: Option<u32>,
    points: &[ClientPoint],
) -> WindowReport {
    unsafe {
        let (surface, parent) = (handle(surface), handle(parent));
        let own_process = GetCurrentProcessId();
        let stray_windows = top_level_windows()
            .into_iter()
            .filter(|&window| window != parent)
            .map(|window| describe(window, parent))
            .filter(|info| {
                info.visible
                    && (info.process_id == own_process || Some(info.process_id) == mpv_process)
            })
            .collect();
        let video_windows = child_windows(surface)
            .into_iter()
            .map(|window| describe(window, parent))
            .collect();
        let hit_tests = points
            .iter()
            .map(|&point| hit_test(parent, surface, point))
            .collect();
        WindowReport {
            surface_visible: IsWindowVisible(surface).as_bool(),
            surface_rect: client_rect_of(surface, parent),
            stray_windows,
            video_windows,
            region: read_region(surface),
            hit_tests,
        }
    }
}

unsafe fn describe(window: HWND, main: HWND) -> WindowInfo {
    unsafe {
        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        let mut class = [0u16; 256];
        let class_len = GetClassNameW(window, &mut class).max(0) as usize;
        let mut title = [0u16; 256];
        let title_len = GetWindowTextW(window, &mut title).max(0) as usize;
        WindowInfo {
            handle: format!("{:#x}", window.0 as usize),
            class_name: String::from_utf16_lossy(&class[..class_len]),
            title: String::from_utf16_lossy(&title[..title_len]),
            process_id,
            visible: IsWindowVisible(window).as_bool(),
            enabled: IsWindowEnabled(window).as_bool(),
            inside_main_window: GetAncestor(window, GA_ROOT) == main,
            rect: client_rect_of(window, main),
        }
    }
}

unsafe fn client_rect_of(window: HWND, main: HWND) -> SurfaceRect {
    unsafe {
        let mut rect = RECT::default();
        let _ = GetWindowRect(window, &mut rect);
        let mut corners = [
            POINT {
                x: rect.left,
                y: rect.top,
            },
            POINT {
                x: rect.right,
                y: rect.bottom,
            },
        ];
        MapWindowPoints(None, Some(main), &mut corners);
        SurfaceRect::from_rect(RECT {
            left: corners[0].x,
            top: corners[0].y,
            right: corners[1].x,
            bottom: corners[1].y,
        })
    }
}

unsafe fn hit_test(main: HWND, surface: HWND, point: ClientPoint) -> HitTest {
    unsafe {
        let mut screen = POINT {
            x: point.x,
            y: point.y,
        };
        let _ = ClientToScreen(main, &mut screen);
        let target = WindowFromPoint(screen);
        let mut class = [0u16; 256];
        let len = GetClassNameW(target, &mut class).max(0) as usize;
        HitTest {
            point,
            class_name: String::from_utf16_lossy(&class[..len]),
            hits_video: target == surface || IsChild(surface, target).as_bool(),
        }
    }
}

unsafe fn read_region(surface: HWND) -> Vec<SurfaceRect> {
    unsafe {
        let region = CreateRectRgn(0, 0, 0, 0);
        let mut rects = Vec::new();
        if GetWindowRgn(surface, region).0 > 1 {
            let size = GetRegionData(region, 0, None);
            if size > 0 {
                let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
                let data = buffer.as_mut_ptr().cast();
                if GetRegionData(region, size, Some(data)) == size {
                    let header = &*(data as *const RGNDATAHEADER);
                    let first = (data as *const u8).add(header.dwSize as usize) as *const RECT;
                    for index in 0..header.nCount as usize {
                        rects.push(SurfaceRect::from_rect(*first.add(index)));
                    }
                }
            }
        }
        let _ = DeleteObject(HRGN(region.0).into());
        rects
    }
}

unsafe fn top_level_windows() -> Vec<HWND> {
    unsafe extern "system" fn collect(window: HWND, list: LPARAM) -> BOOL {
        unsafe { (*(list.0 as *mut Vec<HWND>)).push(window) };
        true.into()
    }
    let mut list: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

unsafe fn child_windows(parent: HWND) -> Vec<HWND> {
    unsafe extern "system" fn collect(window: HWND, list: LPARAM) -> BOOL {
        unsafe { (*(list.0 as *mut Vec<HWND>)).push(window) };
        true.into()
    }
    let mut list: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumChildWindows(
            Some(parent),
            Some(collect),
            LPARAM(&mut list as *mut _ as isize),
        );
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_normal_rectangle() {
        let rect = SurfaceRect {
            x: 240,
            y: 120,
            width: 640,
            height: 360,
        };
        assert_eq!(rect.validate(), Ok(rect));
    }

    #[test]
    fn rejects_empty_or_huge_rectangles() {
        let empty = SurfaceRect {
            x: 0,
            y: 0,
            width: 0,
            height: 10,
        };
        let huge = SurfaceRect {
            x: 0,
            y: 0,
            width: 100_000,
            height: 10,
        };
        assert!(empty.validate().is_err());
        assert!(huge.validate().is_err());
    }

    fn rect(x: i32, y: i32, width: i32, height: i32) -> SurfaceRect {
        SurfaceRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn fully_visible_surface_is_not_clipped() {
        let placement = SurfacePlacement {
            bounds: rect(100, 50, 640, 360),
            visible: rect(100, 50, 640, 360),
        };
        assert_eq!(placement.visible_part(), None);
    }

    #[test]
    fn surface_scrolled_under_the_player_bar_keeps_only_its_visible_part() {
        let placement = SurfacePlacement {
            bounds: rect(100, 500, 640, 360),
            visible: rect(100, 500, 640, 120),
        };
        assert_eq!(placement.visible_part(), Some(rect(0, 0, 640, 120)));
    }

    #[test]
    fn while_moving_only_what_is_visible_in_both_positions_is_shown() {
        let before = SurfacePlacement {
            bounds: rect(100, 776, 580, 326),
            visible: rect(100, 776, 580, 36),
        };
        let after = SurfacePlacement {
            bounds: rect(100, 676, 580, 326),
            visible: rect(100, 676, 580, 136),
        };
        assert_eq!(after.moving_part(&before), rect(0, 0, 580, 36));
        assert_eq!(before.moving_part(&after), rect(0, 0, 580, 36));
    }

    #[test]
    fn surface_scrolled_above_the_view_is_clipped_from_the_top() {
        let placement = SurfacePlacement {
            bounds: rect(100, -200, 640, 360),
            visible: rect(100, 0, 640, 160),
        };
        assert_eq!(placement.visible_part(), Some(rect(0, 200, 640, 160)));
    }
}
