//! Windows Service Daemon and UAC Elevation Manager
//! Provides IPC via Named Pipes, SCM lifecycle management, desktop switching, and SAS (Secure Attention Sequence).

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SERVICE_NAME: &str = "VrVDeskService";
pub const SERVICE_DISPLAY_NAME: &str = "VrV Desk Remote Service";
pub const SERVICE_PIPE_NAME: &str = r"\\.\pipe\VrVDeskServicePipe";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceStatusInfo {
    pub installed: bool,
    pub running: bool,
    pub elevated: bool,
    pub is_service: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeRequest {
    pub cmd: String,
    pub args: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeResponse {
    pub status: String,
    pub elevated: bool,
    pub is_service: bool,
    pub message: Option<String>,
}

#[cfg(windows)]
mod win_impl {
    use super::*;
    use std::ffi::c_void;
    use std::ptr;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, HWND};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::System::Services::*;
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, OpenInputDesktop, SetThreadDesktop, DESKTOP_ACCESS_FLAGS,
        DESKTOP_CONTROL_FLAGS, DESKTOP_CREATEWINDOW, DESKTOP_ENUMERATE, DESKTOP_HOOKCONTROL,
        DESKTOP_JOURNALPLAYBACK, DESKTOP_JOURNALRECORD, DESKTOP_READOBJECTS,
        DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS};
    use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;

    pub fn is_elevated() -> bool {
        unsafe {
            let mut token = HANDLE::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
                return false;
            }

            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = 0u32;
            let res = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut c_void),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );
            let _ = CloseHandle(token);

            if res.is_ok() {
                elevation.TokenIsElevated != 0
            } else {
                false
            }
        }
    }

    pub fn request_elevation(exe_path: Option<&str>, args: Option<&str>) -> Result<(), String> {
        unsafe {
            let exe_to_run = if let Some(p) = exe_path {
                p.to_string()
            } else {
                std::env::current_exe()
                    .map_err(|e| format!("Failed to get current_exe: {}", e))?
                    .to_string_lossy()
                    .to_string()
            };

            let wide_exe: Vec<u16> = exe_to_run.encode_utf16().chain(std::iter::once(0)).collect();
            let wide_verb: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();
            let wide_params: Option<Vec<u16>> = args.map(|a| a.encode_utf16().chain(std::iter::once(0)).collect());

            let mut exec_info = SHELLEXECUTEINFOW {
                cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                fMask: SEE_MASK_NOCLOSEPROCESS,
                hwnd: HWND::default(),
                lpVerb: PCWSTR(wide_verb.as_ptr()),
                lpFile: PCWSTR(wide_exe.as_ptr()),
                lpParameters: if let Some(ref p) = wide_params {
                    PCWSTR(p.as_ptr())
                } else {
                    PCWSTR::null()
                },
                lpDirectory: PCWSTR::null(),
                nShow: SW_NORMAL.0,
                hInstApp: Default::default(),
                lpIDList: ptr::null_mut(),
                lpClass: PCWSTR::null(),
                hkeyClass: Default::default(),
                dwHotKey: 0,
                Anonymous: Default::default(),
                hProcess: Default::default(),
            };

            ShellExecuteExW(&mut exec_info).map_err(|e| format!("ShellExecuteExW runas failed: {:?}", e))?;
            Ok(())
        }
    }

    pub fn trigger_sas() -> Result<(), String> {
        unsafe {
            // Try loading sas.dll
            let sas_dll = LoadLibraryW(w!("sas.dll"));
            if let Ok(module) = sas_dll {
                if !module.is_invalid() {
                    let proc = GetProcAddress(module, windows::core::s!("SendSAS"));
                    if let Some(send_sas_fn) = proc {
                        type SendSasFn = unsafe extern "system" fn(as_user: BOOL);
                        let send_sas: SendSasFn = std::mem::transmute(send_sas_fn);
                        send_sas(BOOL(0));
                        return Ok(());
                    }
                }
            }

            // Fallback: simulate Win+L or notify that SAS requires service privileges
            Err("SendSAS in sas.dll is unavailable or requires Service SYSTEM privileges".to_string())
        }
    }

    pub fn switch_to_input_desktop() -> Result<(), String> {
        unsafe {
            let access = DESKTOP_ACCESS_FLAGS(
                DESKTOP_READOBJECTS.0
                    | DESKTOP_WRITEOBJECTS.0
                    | DESKTOP_SWITCHDESKTOP.0
                    | DESKTOP_ENUMERATE.0
                    | DESKTOP_CREATEWINDOW.0
                    | DESKTOP_HOOKCONTROL.0
                    | DESKTOP_JOURNALPLAYBACK.0
                    | DESKTOP_JOURNALRECORD.0,
            );

            let h_desk = OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, access);
            if let Ok(desktop) = h_desk {
                if !desktop.is_invalid() {
                    let res = SetThreadDesktop(desktop);
                    let _ = CloseDesktop(desktop);
                    if res.is_ok() {
                        return Ok(());
                    } else {
                        return Err(format!("SetThreadDesktop failed: {:?}", res));
                    }
                }
            }
            Err("OpenInputDesktop failed (requires elevated/system token)".to_string())
        }
    }

    pub fn install_service(exe_path: &Path) -> Result<(), String> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)
                .map_err(|e| format!("OpenSCManagerW failed: {:?}", e))?;

            let wide_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let wide_display: Vec<u16> = SERVICE_DISPLAY_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let path_str = format!("\"{}\" --run-service", exe_path.to_string_lossy());
            let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

            let service = CreateServiceW(
                scm,
                PCWSTR(wide_name.as_ptr()),
                PCWSTR(wide_display.as_ptr()),
                SERVICE_ALL_ACCESS,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                PCWSTR(wide_path.as_ptr()),
                PCWSTR::null(),
                None,
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
            );

            let res = service.map(|s| {
                let _ = CloseServiceHandle(s);
            });

            let _ = CloseServiceHandle(scm);
            res.map_err(|e| format!("CreateServiceW failed: {:?}", e))
        }
    }

    pub fn uninstall_service() -> Result<(), String> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)
                .map_err(|e| format!("OpenSCManagerW failed: {:?}", e))?;

            let wide_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let service = OpenServiceW(scm, PCWSTR(wide_name.as_ptr()), SERVICE_ALL_ACCESS);

            let res = if let Ok(s) = service {
                let mut status = SERVICE_STATUS::default();
                let _ = ControlService(s, SERVICE_CONTROL_STOP, &mut status);
                let del_res = DeleteService(s);
                let _ = CloseServiceHandle(s);
                del_res.map_err(|e| format!("DeleteService failed: {:?}", e))
            } else {
                Err("Service not found".to_string())
            };

            let _ = CloseServiceHandle(scm);
            res
        }
    }

    pub fn start_service() -> Result<(), String> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)
                .map_err(|e| format!("OpenSCManagerW failed: {:?}", e))?;

            let wide_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let service = OpenServiceW(scm, PCWSTR(wide_name.as_ptr()), SERVICE_START);

            let res = if let Ok(s) = service {
                let start_res = StartServiceW(s, None);
                let _ = CloseServiceHandle(s);
                start_res.map_err(|e| format!("StartServiceW failed: {:?}", e))
            } else {
                Err("Service not found".to_string())
            };

            let _ = CloseServiceHandle(scm);
            res
        }
    }

    pub fn stop_service() -> Result<(), String> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)
                .map_err(|e| format!("OpenSCManagerW failed: {:?}", e))?;

            let wide_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let service = OpenServiceW(scm, PCWSTR(wide_name.as_ptr()), SERVICE_STOP);

            let res = if let Ok(s) = service {
                let mut status = SERVICE_STATUS::default();
                let stop_res = ControlService(s, SERVICE_CONTROL_STOP, &mut status);
                let _ = CloseServiceHandle(s);
                stop_res.map_err(|e| format!("ControlService stop failed: {:?}", e))
            } else {
                Err("Service not found".to_string())
            };

            let _ = CloseServiceHandle(scm);
            res
        }
    }

    pub fn get_service_status() -> ServiceStatusInfo {
        let elevated = is_elevated();
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT);
            if scm.is_err() {
                return ServiceStatusInfo {
                    installed: false,
                    running: false,
                    elevated,
                    is_service: false,
                };
            }
            let scm = scm.unwrap();
            let wide_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let service = OpenServiceW(scm, PCWSTR(wide_name.as_ptr()), SERVICE_QUERY_STATUS);

            let mut installed = false;
            let mut running = false;

            if let Ok(s) = service {
                installed = true;
                let mut status_process = SERVICE_STATUS_PROCESS::default();
                let mut bytes_needed = 0u32;
                let slice = std::slice::from_raw_parts_mut(
                    &mut status_process as *mut _ as *mut u8,
                    std::mem::size_of::<SERVICE_STATUS_PROCESS>(),
                );
                let q_res = QueryServiceStatusEx(
                    s,
                    SC_STATUS_PROCESS_INFO,
                    Some(slice),
                    &mut bytes_needed,
                );
                if q_res.is_ok() && status_process.dwCurrentState == SERVICE_RUNNING {
                    running = true;
                }
                let _ = CloseServiceHandle(s);
            }

            let _ = CloseServiceHandle(scm);
            ServiceStatusInfo {
                installed,
                running,
                elevated,
                is_service: false,
            }
        }
    }
}

#[cfg(windows)]
pub use win_impl::*;

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(not(windows))]
pub fn request_elevation(_exe_path: Option<&str>, _args: Option<&str>) -> Result<(), String> {
    Err("Elevation only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn trigger_sas() -> Result<(), String> {
    Err("SAS only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn switch_to_input_desktop() -> Result<(), String> {
    Err("Desktop switching only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn install_service(_exe_path: &Path) -> Result<(), String> {
    Err("Windows Service only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn uninstall_service() -> Result<(), String> {
    Err("Windows Service only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn start_service() -> Result<(), String> {
    Err("Windows Service only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn stop_service() -> Result<(), String> {
    Err("Windows Service only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn get_service_status() -> ServiceStatusInfo {
    ServiceStatusInfo {
        installed: false,
        running: false,
        elevated: false,
        is_service: false,
    }
}

// ---------------------------------------------------------------------------
// Named Pipe IPC (Client & Server)
// ---------------------------------------------------------------------------

pub async fn send_pipe_command(cmd_json: &str) -> Result<String, String> {
    #[cfg(windows)]
    {
        let req: PipeRequest = serde_json::from_str(cmd_json)
            .map_err(|e| format!("Invalid JSON request: {}", e))?;
        let resp = pipe_ipc::send_command(&req).await?;
        serde_json::to_string(&resp).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = cmd_json;
        Err("Named pipe IPC only supported on Windows".to_string())
    }
}

pub mod pipe_ipc {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};

    pub async fn send_command(req: &PipeRequest) -> Result<PipeResponse, String> {
        let client = ClientOptions::new()
            .open(SERVICE_PIPE_NAME)
            .map_err(|e| format!("Failed to connect to service pipe {}: {}", SERVICE_PIPE_NAME, e))?;

        let mut client = client;
        let serialized = serde_json::to_string(req).map_err(|e| e.to_string())?;
        client
            .write_all(serialized.as_bytes())
            .await
            .map_err(|e| format!("Pipe write error: {}", e))?;
        client.flush().await.map_err(|e| format!("Pipe flush error: {}", e))?;

        // Read response
        let mut buffer = vec![0u8; 4096];
        let n = client
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Pipe read error: {}", e))?;
        if n == 0 {
            return Err("Empty response from pipe".to_string());
        }

        let resp_str = String::from_utf8_lossy(&buffer[..n]);
        let resp: PipeResponse = serde_json::from_str(&resp_str).map_err(|e| e.to_string())?;
        Ok(resp)
    }

    pub async fn run_server(is_service: bool) -> Result<(), String> {
        let mut first = true;
        loop {
            let server = if first {
                first = false;
                ServerOptions::new()
                    .first_pipe_instance(true)
                    .create(SERVICE_PIPE_NAME)
                    .map_err(|e| format!("Failed to create pipe server: {}", e))?
            } else {
                ServerOptions::new()
                    .create(SERVICE_PIPE_NAME)
                    .map_err(|e| format!("Failed to create pipe instance: {}", e))?
            };

            server.connect().await.map_err(|e| format!("Pipe connect error: {}", e))?;

            let mut server = server;
            let is_elevated_now = is_elevated();

            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                if let Ok(n) = server.read(&mut buf).await {
                    if n > 0 {
                        let text = String::from_utf8_lossy(&buf[..n]);
                        let resp = if let Ok(req) = serde_json::from_str::<PipeRequest>(&text) {
                            match req.cmd.as_str() {
                                "ping" => PipeResponse {
                                    status: "ok".to_string(),
                                    elevated: is_elevated_now,
                                    is_service,
                                    message: Some("pong".to_string()),
                                },
                                "is_elevated" => PipeResponse {
                                    status: "ok".to_string(),
                                    elevated: is_elevated_now,
                                    is_service,
                                    message: None,
                                },
                                "sas" => {
                                    let sas_res = trigger_sas();
                                    PipeResponse {
                                        status: if sas_res.is_ok() { "ok".to_string() } else { "error".to_string() },
                                        elevated: is_elevated_now,
                                        is_service,
                                        message: sas_res.err(),
                                    }
                                }
                                "switch_desktop" => {
                                    let sw_res = switch_to_input_desktop();
                                    PipeResponse {
                                        status: if sw_res.is_ok() { "ok".to_string() } else { "error".to_string() },
                                        elevated: is_elevated_now,
                                        is_service,
                                        message: sw_res.err(),
                                    }
                                }
                                "elevate_host" => {
                                    let res = request_elevation(req.args.as_deref(), None);
                                    PipeResponse {
                                        status: if res.is_ok() { "ok".to_string() } else { "error".to_string() },
                                        elevated: is_elevated_now,
                                        is_service,
                                        message: res.err(),
                                    }
                                }
                                "install_virtual_display" => {
                                    let res = crate::virtual_display::install_driver(req.args.as_deref());
                                    PipeResponse {
                                        status: if res.is_ok() { "ok".to_string() } else { "error".to_string() },
                                        elevated: is_elevated_now,
                                        is_service,
                                        message: match res {
                                            Ok(m) => Some(m),
                                            Err(e) => Some(e),
                                        },
                                    }
                                }
                                "uninstall_virtual_display" => {
                                    let res = crate::virtual_display::uninstall_driver();
                                    PipeResponse {
                                        status: if res.is_ok() { "ok".to_string() } else { "error".to_string() },
                                        elevated: is_elevated_now,
                                        is_service,
                                        message: match res {
                                            Ok(m) => Some(m),
                                            Err(e) => Some(e),
                                        },
                                    }
                                }
                                other => PipeResponse {
                                    status: "error".to_string(),
                                    elevated: is_elevated_now,
                                    is_service,
                                    message: Some(format!("Unknown command: {}", other)),
                                },
                            }
                        } else {
                            PipeResponse {
                                status: "error".to_string(),
                                elevated: is_elevated_now,
                                is_service,
                                message: Some("Invalid JSON request".to_string()),
                            }
                        };

                        if let Ok(resp_json) = serde_json::to_string(&resp) {
                            let _ = server.write_all(resp_json.as_bytes()).await;
                            let _ = server.flush().await;
                        }
                    }
                }
            });
        }
    }
}
