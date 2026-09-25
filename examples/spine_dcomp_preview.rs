#![cfg_attr(not(target_os = "windows"), allow(dead_code, unused_imports))]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("spine_dcomp_preview currently requires Windows.");
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
#[path = "../src/render/spine/mod.rs"]
mod spine;

#[cfg(target_os = "windows")]
mod windows_preview {
    use std::{
        path::Path,
        time::{Duration, Instant},
    };

    use crate::spine::{SpineDcompRenderer, SpineRuntime};
    use windows::{
        Win32::{
            Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
            System::LibraryLoader::GetModuleHandleW,
            UI::{
                Input::KeyboardAndMouse::VK_ESCAPE,
                WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG,
                    PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassW, SW_SHOWNOACTIVATE,
                    ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY,
                    WM_KEYDOWN, WM_QUIT, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP,
                    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
                },
            },
        },
        core::w,
    };

    const WIDTH: u32 = 520;
    const HEIGHT: u32 = 620;
    const WINDOW_X: i32 = 120;
    const WINDOW_Y: i32 = 120;

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_KEYDOWN if wparam.0 as u16 == VK_ESCAPE.0 => {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    fn create_window() -> Result<HWND, String> {
        let module = unsafe { GetModuleHandleW(None) }
            .map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
        let instance = HINSTANCE(module.0);
        let class_name = w!("SenaSpineDcompPreviewWindow");

        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name,
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(format!(
                "RegisterClassW failed: {}",
                windows::core::Error::from_thread()
            ));
        }

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(
                    WS_EX_TOOLWINDOW.0
                        | WS_EX_TOPMOST.0
                        | WS_EX_NOACTIVATE.0
                        | WS_EX_NOREDIRECTIONBITMAP.0,
                ),
                class_name,
                w!("Sena Spine 3.8 — DirectComposition R2B"),
                WS_POPUP,
                WINDOW_X,
                WINDOW_Y,
                WIDTH as i32,
                HEIGHT as i32,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .map_err(|error| format!("CreateWindowExW failed: {error}"))?;

        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(hwnd)
    }

    fn parse_args() -> (String, Option<u64>) {
        let mut animation = "idle".to_string();
        let mut max_frames = None;
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--animation" => {
                    if let Some(value) = args.next() {
                        animation = value;
                    }
                }
                "--frames" => {
                    max_frames = args.next().and_then(|value| value.parse().ok());
                }
                _ => {}
            }
        }

        (animation, max_frames)
    }

    pub fn run() -> Result<(), String> {
        let (animation, max_frames) = parse_args();
        let export = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("third_party")
            .join("spine-runtimes")
            .join("examples")
            .join("spineboy")
            .join("export");
        let skeleton = export.join("spineboy-pro.json");
        let atlas = export.join("spineboy-pma.atlas");

        let mut spine = SpineRuntime::from_files(&skeleton, &atlas, 1.0)?;
        spine.set_animation(0, &animation, true)?;

        let hwnd = create_window()?;
        let mut renderer = SpineDcompRenderer::new(hwnd, &atlas, WIDTH, HEIGHT)?;

        let mut rendered_frames = 0u64;
        let mut message = MSG::default();
        let mut last_tick = Instant::now();
        let frame_time = Duration::from_micros(16_667);

        'running: loop {
            let frame_started = Instant::now();

            while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                if message.message == WM_QUIT {
                    break 'running;
                }
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            let now = Instant::now();
            let delta = now.duration_since(last_tick).as_secs_f32().min(0.1);
            last_tick = now;
            spine.update(delta);
            let frame = spine.render_frame()?;
            renderer.render(&frame)?;

            rendered_frames += 1;
            if max_frames.is_some_and(|limit| rendered_frames >= limit) {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                break;
            }

            if let Some(remaining) = frame_time.checked_sub(frame_started.elapsed()) {
                std::thread::sleep(remaining);
            }
        }

        eprintln!(
            "Sena Spine DirectComposition preview rendered {rendered_frames} frames ({animation})."
        );
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn main() {
    if let Err(error) = windows_preview::run() {
        eprintln!("Sena Spine DirectComposition preview failed: {error}");
        std::process::exit(1);
    }
}
