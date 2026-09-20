#![windows_subsystem = "windows"]

use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

// Embedded binaries
static VRV_DESK_BYTES: &[u8] = include_bytes!("../../../target/release/vrv_desk.exe");

#[derive(Clone, Copy, PartialEq, Eq)]
enum WizardPage {
    Welcome = 0,
    Destination = 1,
    Tasks = 2,
    Ready = 3,
    Installing = 4,
    Finished = 5,
}

struct SetupState {
    page: WizardPage,
    dest_path: PathBuf,
    create_desktop_icon: bool,
    create_start_menu: bool,
    launch_app: bool,
    install_progress: Arc<AtomicU32>, // 0 to 100
    install_status: Arc<std::sync::Mutex<String>>,
    hwnd_dest_edit: HWND,
    hwnd_btn_browse: HWND,
    hwnd_chk_desktop: HWND,
    hwnd_chk_startmenu: HWND,
    hwnd_chk_launch: HWND,
    hwnd_btn_back: HWND,
    hwnd_btn_next: HWND,
    hwnd_btn_cancel: HWND,
}

static mut SETUP_STATE: Option<SetupState> = None;

const ID_BTN_BACK: usize = 2001;
const ID_BTN_NEXT: usize = 2002;
const ID_BTN_CANCEL: usize = 2003;
const ID_BTN_BROWSE: usize = 2004;
const ID_EDIT_DEST: usize = 2005;
const ID_CHK_DESKTOP: usize = 2006;
const ID_CHK_STARTMENU: usize = 2007;
const ID_CHK_LAUNCH: usize = 2008;

const COLOR_WIZARD_BG: u32 = 0x00F0F0F0;
const COLOR_SIDEBAR_BG: u32 = 0x00663311; // Deep Navy
const COLOR_WHITE: u32 = 0x00FFFFFF;
const COLOR_TEXT_MAIN: u32 = 0x00202020;
const COLOR_TEXT_MUTED: u32 = 0x00606060;
const COLOR_ACCENT: u32 = 0x00D27000; // Cyan/Blue

unsafe fn make_font(height: i32, weight: i32, face: PCWSTR) -> HFONT {
    CreateFontW(
        height, 0, 0, 0, weight, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        face,
    )
}

fn get_default_install_dir() -> PathBuf {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_app_data).join("Programs").join("VrV Desk")
    } else {
        PathBuf::from(r"C:\Program Files\VrV Desk")
    }
}

fn create_shortcut(target_exe: &Path, shortcut_path: &Path, description: &str) -> bool {
    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}'); $s.TargetPath = '{}'; $s.Description = '{}'; $s.WorkingDirectory = '{}'; $s.Save()",
        shortcut_path.display(),
        target_exe.display(),
        description,
        target_exe.parent().unwrap_or(Path::new("")).display()
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();

    matches!(output, Ok(o) if o.status.success())
}

fn register_uninstaller(dest_dir: &Path, exe_path: &Path) {
    let uninst_key = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\VrVDesk";
    let dest_str = dest_dir.display().to_string();
    let exe_str = exe_path.display().to_string();
    let ps_reg = format!(
        "New-Item -Path 'HKCU:\\{k}' -Force | Out-Null; \
         Set-ItemProperty -Path 'HKCU:\\{k}' -Name 'DisplayName' -Value 'VrV Desk'; \
         Set-ItemProperty -Path 'HKCU:\\{k}' -Name 'DisplayVersion' -Value '0.14.0'; \
         Set-ItemProperty -Path 'HKCU:\\{k}' -Name 'Publisher' -Value 'VrV Desk Team'; \
         Set-ItemProperty -Path 'HKCU:\\{k}' -Name 'InstallLocation' -Value '{d}'; \
         Set-ItemProperty -Path 'HKCU:\\{k}' -Name 'DisplayIcon' -Value '{e}'; \
         Set-ItemProperty -Path 'HKCU:\\{k}' -Name 'UninstallString' -Value 'powershell -NoProfile -Command Remove-Item -Recurse -Force \"{d}\"; Remove-Item -Path \"HKCU:\\{k}\" -Recurse -Force'",
        k = uninst_key,
        d = dest_str,
        e = exe_str
    );

    let _ = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_reg])
        .output();
}

fn perform_install(
    dest_dir: PathBuf,
    create_desktop: bool,
    create_start: bool,
    progress: Arc<AtomicU32>,
    status: Arc<std::sync::Mutex<String>>,
    hwnd: HWND,
) {
    std::thread::spawn(move || {
        // Step 1: Create Destination Directory
        {
            let mut s = status.lock().unwrap();
            *s = "Creating destination directory...".to_string();
        }
        progress.store(15, Ordering::Relaxed);
        unsafe { InvalidateRect(hwnd, None, FALSE); }
        std::thread::sleep(std::time::Duration::from_millis(300));

        if let Err(e) = fs::create_dir_all(&dest_dir) {
            let mut s = status.lock().unwrap();
            *s = format!("Failed to create folder: {}", e);
            return;
        }

        // Step 2: Extract & Write vrv_desk.exe
        {
            let mut s = status.lock().unwrap();
            *s = "Extracting vrv_desk.exe...".to_string();
        }
        progress.store(40, Ordering::Relaxed);
        unsafe { InvalidateRect(hwnd, None, FALSE); }
        std::thread::sleep(std::time::Duration::from_millis(400));

        let target_exe = dest_dir.join("vrv_desk.exe");
        if let Err(e) = fs::write(&target_exe, VRV_DESK_BYTES) {
            let mut s = status.lock().unwrap();
            *s = format!("Failed to write executable: {}", e);
            return;
        }

        // Step 3: Create Desktop Shortcut
        if create_desktop {
            {
                let mut s = status.lock().unwrap();
                *s = "Creating desktop shortcut...".to_string();
            }
            progress.store(65, Ordering::Relaxed);
            unsafe { InvalidateRect(hwnd, None, FALSE); }
            std::thread::sleep(std::time::Duration::from_millis(300));

            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                let desktop_dir = PathBuf::from(user_profile).join("Desktop");
                let lnk_path = desktop_dir.join("VrV Desk.lnk");
                create_shortcut(&target_exe, &lnk_path, "VrV Desk - Screen Mirror & Remote Control");
            }
        }

        // Step 4: Create Start Menu Shortcut
        if create_start {
            {
                let mut s = status.lock().unwrap();
                *s = "Creating Start Menu shortcut...".to_string();
            }
            progress.store(80, Ordering::Relaxed);
            unsafe { InvalidateRect(hwnd, None, FALSE); }
            std::thread::sleep(std::time::Duration::from_millis(300));

            if let Ok(app_data) = std::env::var("APPDATA") {
                let start_menu_dir = PathBuf::from(app_data)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs")
                    .join("VrV Desk");
                let _ = fs::create_dir_all(&start_menu_dir);
                let lnk_path = start_menu_dir.join("VrV Desk.lnk");
                create_shortcut(&target_exe, &lnk_path, "VrV Desk");
            }
        }

        // Step 5: Register Uninstaller in Windows Registry
        {
            let mut s = status.lock().unwrap();
            *s = "Registering application in Windows...".to_string();
        }
        progress.store(95, Ordering::Relaxed);
        unsafe { InvalidateRect(hwnd, None, FALSE); }
        register_uninstaller(&dest_dir, &target_exe);
        std::thread::sleep(std::time::Duration::from_millis(400));

        // Complete!
        progress.store(100, Ordering::Relaxed);
        {
            let mut s = status.lock().unwrap();
            *s = "Installation completed successfully!".to_string();
        }

        // Switch to Finished page
        unsafe {
            if let Some(ref mut state) = SETUP_STATE {
                state.page = WizardPage::Finished;
                update_page_controls(state);
            }
            InvalidateRect(hwnd, None, TRUE);
        }
    });
}

unsafe fn update_page_controls(state: &mut SetupState) {
    let show_dest = state.page == WizardPage::Destination;
    let show_tasks = state.page == WizardPage::Tasks;
    let show_finish = state.page == WizardPage::Finished;

    let sw_dest = if show_dest { SW_SHOW } else { SW_HIDE };
    let sw_tasks = if show_tasks { SW_SHOW } else { SW_HIDE };
    let sw_finish = if show_finish { SW_SHOW } else { SW_HIDE };

    let _ = ShowWindow(state.hwnd_dest_edit, sw_dest);
    let _ = ShowWindow(state.hwnd_btn_browse, sw_dest);
    let _ = ShowWindow(state.hwnd_chk_desktop, sw_tasks);
    let _ = ShowWindow(state.hwnd_chk_startmenu, sw_tasks);
    let _ = ShowWindow(state.hwnd_chk_launch, sw_finish);

    // Update button labels & enable states
    match state.page {
        WizardPage::Welcome => {
            let _ = EnableWindow(state.hwnd_btn_back, false);
            let _ = SetWindowTextW(state.hwnd_btn_next, w!("Next >"));
            let _ = EnableWindow(state.hwnd_btn_next, true);
        }
        WizardPage::Destination | WizardPage::Tasks => {
            let _ = EnableWindow(state.hwnd_btn_back, true);
            let _ = SetWindowTextW(state.hwnd_btn_next, w!("Next >"));
            let _ = EnableWindow(state.hwnd_btn_next, true);
        }
        WizardPage::Ready => {
            let _ = EnableWindow(state.hwnd_btn_back, true);
            let _ = SetWindowTextW(state.hwnd_btn_next, w!("Install"));
            let _ = EnableWindow(state.hwnd_btn_next, true);
        }
        WizardPage::Installing => {
            let _ = EnableWindow(state.hwnd_btn_back, false);
            let _ = EnableWindow(state.hwnd_btn_next, false);
            let _ = EnableWindow(state.hwnd_btn_cancel, false);
        }
        WizardPage::Finished => {
            let _ = EnableWindow(state.hwnd_btn_back, false);
            let _ = SetWindowTextW(state.hwnd_btn_next, w!("Finish"));
            let _ = EnableWindow(state.hwnd_btn_next, true);
            let _ = EnableWindow(state.hwnd_btn_cancel, false);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let state = SETUP_STATE.as_mut().unwrap();
            let hinst = GetModuleHandleW(None).unwrap();

            // Destination Edit Box & Browse Button
            let dest_wstr: Vec<u16> = state.dest_path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
            state.hwnd_dest_edit = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                PCWSTR(dest_wstr.as_ptr()),
                WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                185, 120, 235, 24,
                hwnd, HMENU(ID_EDIT_DEST as _), hinst, None,
            );

            state.hwnd_btn_browse = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Browse..."),
                WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                426, 120, 74, 24,
                hwnd, HMENU(ID_BTN_BROWSE as _), hinst, None,
            );

            // Tasks Checkboxes
            state.hwnd_chk_desktop = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Create a desktop shortcut"),
                WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                185, 120, 300, 24,
                hwnd, HMENU(ID_CHK_DESKTOP as _), hinst, None,
            );
            SendMessageW(state.hwnd_chk_desktop, BM_SETCHECK, WPARAM(1), LPARAM(0));

            state.hwnd_chk_startmenu = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Create a Start Menu shortcut"),
                WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                185, 150, 300, 24,
                hwnd, HMENU(ID_CHK_STARTMENU as _), hinst, None,
            );
            SendMessageW(state.hwnd_chk_startmenu, BM_SETCHECK, WPARAM(1), LPARAM(0));

            // Finished Page Checkbox
            state.hwnd_chk_launch = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Launch VrV Desk now"),
                WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                185, 200, 300, 24,
                hwnd, HMENU(ID_CHK_LAUNCH as _), hinst, None,
            );
            SendMessageW(state.hwnd_chk_launch, BM_SETCHECK, WPARAM(1), LPARAM(0));

            // Standard Navigation Buttons
            state.hwnd_btn_back = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("< Back"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                260, 320, 75, 26,
                hwnd, HMENU(ID_BTN_BACK as _), hinst, None,
            );

            state.hwnd_btn_next = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Next >"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
                345, 320, 75, 26,
                hwnd, HMENU(ID_BTN_NEXT as _), hinst, None,
            );

            state.hwnd_btn_cancel = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Cancel"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                430, 320, 75, 26,
                hwnd, HMENU(ID_BTN_CANCEL as _), hinst, None,
            );

            update_page_controls(state);
            LRESULT(0)
        }

        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as usize;
            if let Some(ref mut state) = SETUP_STATE {
                match id {
                    ID_BTN_BACK => {
                        match state.page {
                            WizardPage::Destination => state.page = WizardPage::Welcome,
                            WizardPage::Tasks => state.page = WizardPage::Destination,
                            WizardPage::Ready => state.page = WizardPage::Tasks,
                            _ => {}
                        }
                        update_page_controls(state);
                        InvalidateRect(hwnd, None, TRUE);
                    }
                    ID_BTN_NEXT => {
                        match state.page {
                            WizardPage::Welcome => {
                                state.page = WizardPage::Destination;
                                update_page_controls(state);
                                InvalidateRect(hwnd, None, TRUE);
                            }
                            WizardPage::Destination => {
                                let mut buf = [0u16; 512];
                                let len = GetWindowTextW(state.hwnd_dest_edit, &mut buf);
                                if len > 0 {
                                    let raw = String::from_utf16_lossy(&buf[..len as usize]);
                                    let trimmed = raw.trim();
                                    if !trimmed.is_empty() {
                                        state.dest_path = PathBuf::from(trimmed);
                                    }
                                }
                                state.page = WizardPage::Tasks;
                                update_page_controls(state);
                                InvalidateRect(hwnd, None, TRUE);
                            }
                            WizardPage::Tasks => {
                                state.create_desktop_icon = SendMessageW(state.hwnd_chk_desktop, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                                state.create_start_menu = SendMessageW(state.hwnd_chk_startmenu, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                                state.page = WizardPage::Ready;
                                update_page_controls(state);
                                InvalidateRect(hwnd, None, TRUE);
                            }
                            WizardPage::Ready => {
                                state.page = WizardPage::Installing;
                                update_page_controls(state);
                                InvalidateRect(hwnd, None, TRUE);

                                perform_install(
                                    state.dest_path.clone(),
                                    state.create_desktop_icon,
                                    state.create_start_menu,
                                    state.install_progress.clone(),
                                    state.install_status.clone(),
                                    hwnd,
                                );
                            }
                            WizardPage::Finished => {
                                let launch = SendMessageW(state.hwnd_chk_launch, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1;
                                if launch {
                                    let exe_path = state.dest_path.join("vrv_desk.exe");
                                    let _ = Command::new(exe_path).spawn();
                                }
                                PostQuitMessage(0);
                            }
                            _ => {}
                        }
                    }
                    ID_BTN_CANCEL => {
                        let res = MessageBoxW(
                            hwnd,
                            w!("Are you sure you want to exit the VrV Desk Setup Wizard?"),
                            w!("Exit Setup"),
                            MB_YESNO | MB_ICONQUESTION,
                        );
                        if res == IDYES {
                            PostQuitMessage(0);
                        }
                    }
                    ID_BTN_BROWSE => {
                        let current_path = {
                            let mut buf = [0u16; 512];
                            let len = GetWindowTextW(state.hwnd_dest_edit, &mut buf);
                            if len > 0 {
                                String::from_utf16_lossy(&buf[..len as usize])
                            } else {
                                state.dest_path.to_string_lossy().to_string()
                            }
                        };
                        let ps_cmd = format!(
                            "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; $f.Description = 'Select Installation Folder for VrV Desk'; $f.SelectedPath = '{}'; if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ Write-Output $f.SelectedPath }}",
                            current_path.replace('\'', "''")
                        );
                        if let Ok(output) = Command::new("powershell")
                            .args(["-NoProfile", "-Command", &ps_cmd])
                            .output()
                        {
                            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            if !selected.is_empty() {
                                let mut target_folder = PathBuf::from(selected);
                                if !target_folder.ends_with("VrV Desk") {
                                    target_folder = target_folder.join("VrV Desk");
                                }
                                let wstr: Vec<u16> = target_folder.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
                                let _ = SetWindowTextW(state.hwnd_dest_edit, PCWSTR(wstr.as_ptr()));
                                state.dest_path = target_folder;
                            }
                        }
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            if let Some(ref state) = SETUP_STATE {
                // Background
                let brush_bg = CreateSolidBrush(COLORREF(COLOR_WIZARD_BG));
                let mut client_rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut client_rect);
                FillRect(hdc, &client_rect, brush_bg);
                let _ = DeleteObject(brush_bg);

                // Left Sidebar (Welcome & Finished full height, or standard pages)
                let sidebar_rect = RECT { left: 0, top: 0, right: 165, bottom: 310 };
                let brush_sidebar = CreateSolidBrush(COLORREF(COLOR_SIDEBAR_BG));
                FillRect(hdc, &sidebar_rect, brush_sidebar);
                let _ = DeleteObject(brush_sidebar);

                // Sidebar Brand Text
                let font_side = make_font(20, FW_BOLD.0 as i32, w!("Segoe UI"));
                let old_font = SelectObject(hdc, font_side);
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(COLOR_WHITE));
                let mut side_title_rect = RECT { left: 15, top: 30, right: 150, bottom: 60 };
                let mut side_title: Vec<u16> = "VrV Desk".encode_utf16().collect();
                DrawTextW(hdc, &mut side_title, &mut side_title_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                let font_side_sub = make_font(12, FW_NORMAL.0 as i32, w!("Segoe UI"));
                SelectObject(hdc, font_side_sub);
                SetTextColor(hdc, COLORREF(0x00D0D0D0));
                let mut side_sub_rect = RECT { left: 15, top: 60, right: 150, bottom: 120 };
                let mut side_sub: Vec<u16> = "Ultra-low latency\nRemote Desktop\n& Mirroring".encode_utf16().collect();
                DrawTextW(hdc, &mut side_sub, &mut side_sub_rect, DT_LEFT | DT_WORDBREAK);

                // Separator Line above bottom buttons
                let pen_sep = CreatePen(PS_SOLID, 1, COLORREF(0x00D0D0D0));
                let old_pen = SelectObject(hdc, pen_sep);
                MoveToEx(hdc, 0, 310, None);
                LineTo(hdc, client_rect.right, 310);
                SelectObject(hdc, old_pen);
                let _ = DeleteObject(pen_sep);

                // Right Panel Content per page
                match state.page {
                    WizardPage::Welcome => {
                        let font_h1 = make_font(16, FW_BOLD.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_h1);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MAIN));
                        let mut h1_rect = RECT { left: 185, top: 25, right: 500, bottom: 70 };
                        let mut h1: Vec<u16> = "Welcome to the VrV Desk\nSetup Wizard".encode_utf16().collect();
                        DrawTextW(hdc, &mut h1, &mut h1_rect, DT_LEFT | DT_WORDBREAK);

                        let font_body = make_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_body);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
                        let mut body_rect = RECT { left: 185, top: 80, right: 500, bottom: 280 };
                        let mut body: Vec<u16> = "This wizard will install VrV Desk v0.14.0 on your computer.\n\nFeaturing DXGI GPU capture, MFT Hardware H.264, WASAPI loopback audio, and zero-config LAN discovery.\n\nClick Next to continue, or Cancel to exit Setup.".encode_utf16().collect();
                        DrawTextW(hdc, &mut body, &mut body_rect, DT_LEFT | DT_WORDBREAK);
                    }

                    WizardPage::Destination => {
                        let font_h1 = make_font(14, FW_BOLD.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_h1);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MAIN));
                        let mut h1_rect = RECT { left: 185, top: 20, right: 500, bottom: 40 };
                        let mut h1: Vec<u16> = "Select Destination Location".encode_utf16().collect();
                        DrawTextW(hdc, &mut h1, &mut h1_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                        let font_body = make_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_body);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
                        let mut body_rect = RECT { left: 185, top: 45, right: 500, bottom: 90 };
                        let mut body: Vec<u16> = "Where should VrV Desk be installed?\n\nSetup will install VrV Desk into the following folder:".encode_utf16().collect();
                        DrawTextW(hdc, &mut body, &mut body_rect, DT_LEFT | DT_WORDBREAK);

                        let mut space_rect = RECT { left: 185, top: 160, right: 500, bottom: 180 };
                        let mut space: Vec<u16> = "At least 35.0 MB of free disk space is required.".encode_utf16().collect();
                        DrawTextW(hdc, &mut space, &mut space_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
                    }

                    WizardPage::Tasks => {
                        let font_h1 = make_font(14, FW_BOLD.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_h1);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MAIN));
                        let mut h1_rect = RECT { left: 185, top: 20, right: 500, bottom: 40 };
                        let mut h1: Vec<u16> = "Select Additional Tasks".encode_utf16().collect();
                        DrawTextW(hdc, &mut h1, &mut h1_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                        let font_body = make_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_body);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
                        let mut body_rect = RECT { left: 185, top: 45, right: 500, bottom: 90 };
                        let mut body: Vec<u16> = "Which additional tasks should be performed?\n\nSelect the tasks you would like Setup to perform:".encode_utf16().collect();
                        DrawTextW(hdc, &mut body, &mut body_rect, DT_LEFT | DT_WORDBREAK);
                    }

                    WizardPage::Ready => {
                        let font_h1 = make_font(14, FW_BOLD.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_h1);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MAIN));
                        let mut h1_rect = RECT { left: 185, top: 20, right: 500, bottom: 40 };
                        let mut h1: Vec<u16> = "Ready to Install".encode_utf16().collect();
                        DrawTextW(hdc, &mut h1, &mut h1_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                        let font_body = make_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_body);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
                        let summary = format!(
                            "Setup is now ready to install VrV Desk.\n\nDestination folder:\n  {}\n\nTasks:\n  - {}\n  - {}",
                            state.dest_path.display(),
                            if state.create_desktop_icon { "Create desktop shortcut" } else { "No desktop shortcut" },
                            if state.create_start_menu { "Create Start Menu shortcut" } else { "No Start Menu shortcut" }
                        );
                        let mut body_rect = RECT { left: 185, top: 50, right: 500, bottom: 250 };
                        let mut body: Vec<u16> = summary.encode_utf16().collect();
                        DrawTextW(hdc, &mut body, &mut body_rect, DT_LEFT | DT_WORDBREAK);
                    }

                    WizardPage::Installing => {
                        let font_h1 = make_font(14, FW_BOLD.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_h1);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MAIN));
                        let mut h1_rect = RECT { left: 185, top: 20, right: 500, bottom: 40 };
                        let mut h1: Vec<u16> = "Installing VrV Desk...".encode_utf16().collect();
                        DrawTextW(hdc, &mut h1, &mut h1_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                        // Status message
                        let font_body = make_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_body);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
                        let curr_status = state.install_status.lock().unwrap().clone();
                        let mut status_rect = RECT { left: 185, top: 90, right: 500, bottom: 110 };
                        let mut status_w: Vec<u16> = curr_status.encode_utf16().collect();
                        DrawTextW(hdc, &mut status_w, &mut status_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                        // Progress Bar Container
                        let bar_border = RECT { left: 185, top: 120, right: 500, bottom: 142 };
                        let brush_frame = CreateSolidBrush(COLORREF(0x00E0E0E0));
                        FillRect(hdc, &bar_border, brush_frame);
                        let _ = DeleteObject(brush_frame);

                        // Progress Fill
                        let pct = state.install_progress.load(Ordering::Relaxed).min(100);
                        let fill_w = ((500 - 185) * pct as i32) / 100;
                        if fill_w > 0 {
                            let bar_fill = RECT { left: 185, top: 120, right: 185 + fill_w, bottom: 142 };
                            let brush_fill = CreateSolidBrush(COLORREF(COLOR_ACCENT));
                            FillRect(hdc, &bar_fill, brush_fill);
                            let _ = DeleteObject(brush_fill);
                        }
                    }

                    WizardPage::Finished => {
                        let font_h1 = make_font(16, FW_BOLD.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_h1);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MAIN));
                        let mut h1_rect = RECT { left: 185, top: 25, right: 500, bottom: 70 };
                        let mut h1: Vec<u16> = "Completing the VrV Desk\nSetup Wizard".encode_utf16().collect();
                        DrawTextW(hdc, &mut h1, &mut h1_rect, DT_LEFT | DT_WORDBREAK);

                        let font_body = make_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
                        SelectObject(hdc, font_body);
                        SetTextColor(hdc, COLORREF(COLOR_TEXT_MUTED));
                        let mut body_rect = RECT { left: 185, top: 80, right: 500, bottom: 180 };
                        let mut body: Vec<u16> = "VrV Desk has been successfully installed on your computer.\n\nThe application may be launched by selecting the installed shortcuts or checking the option below.\n\nClick Finish to exit Setup.".encode_utf16().collect();
                        DrawTextW(hdc, &mut body, &mut body_rect, DT_LEFT | DT_WORDBREAK);
                    }
                }

                SelectObject(hdc, old_font);
            }

            EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    let dest_path = get_default_install_dir();

    unsafe {
        SETUP_STATE = Some(SetupState {
            page: WizardPage::Welcome,
            dest_path,
            create_desktop_icon: true,
            create_start_menu: true,
            launch_app: true,
            install_progress: Arc::new(AtomicU32::new(0)),
            install_status: Arc::new(std::sync::Mutex::new("Preparing installation...".to_string())),
            hwnd_dest_edit: HWND::default(),
            hwnd_btn_browse: HWND::default(),
            hwnd_chk_desktop: HWND::default(),
            hwnd_chk_startmenu: HWND::default(),
            hwnd_chk_launch: HWND::default(),
            hwnd_btn_back: HWND::default(),
            hwnd_btn_next: HWND::default(),
            hwnd_btn_cancel: HWND::default(),
        });

        let hinstance = GetModuleHandleW(None).unwrap();
        let class_name = w!("VrVDeskSetupWizard");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(COLOR_WINDOW.0 as _),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class_name,
            hIconSm: HICON::default(),
        };

        RegisterClassExW(&wc);

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let win_w = 540;
        let win_h = 390;
        let pos_x = (screen_w - win_w) / 2;
        let pos_y = (screen_h - win_h) / 2;

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Setup - VrV Desk"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            pos_x,
            pos_y,
            win_w,
            win_h,
            None,
            None,
            hinstance,
            None,
        );

        if hwnd.0 == 0 {
            return;
        }

        // Enable Windows 11 DWM rounded corners
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(33), // DWMWA_WINDOW_CORNER_PREFERENCE
            &3u32 as *const _ as *const c_void,
            4,
        );

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
