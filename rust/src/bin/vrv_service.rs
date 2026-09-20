//! VrV Desk Windows Service Daemon (`vrv_service.exe`)
//! Runs as a background Windows Service (SYSTEM) or standalone daemon to support
//! Secure Desktop switching, UAC prompt bypass, SAS (Ctrl+Alt+Del), and process elevation.

use mirror_core::service_manager::*;
use std::env;
use std::path::PathBuf;

#[cfg(windows)]
use windows::core::{PCWSTR, PWSTR};
#[cfg(windows)]
use windows::Win32::System::Services::*;

#[cfg(windows)]
static mut SERVICE_STATUS_HANDLE: Option<SERVICE_STATUS_HANDLE> = None;

#[cfg(windows)]
unsafe extern "system" fn service_handler(control: u32) {
    match control {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            if let Some(handle) = SERVICE_STATUS_HANDLE {
                let status = SERVICE_STATUS {
                    dwServiceType: SERVICE_WIN32_OWN_PROCESS,
                    dwCurrentState: SERVICE_STOPPED,
                    dwControlsAccepted: 0,
                    dwWin32ExitCode: 0,
                    dwServiceSpecificExitCode: 0,
                    dwCheckPoint: 0,
                    dwWaitHint: 0,
                };
                let _ = SetServiceStatus(handle, &status);
            }
        }
        _ => {}
    }
}

#[cfg(windows)]
unsafe extern "system" fn service_main(_argc: u32, _argv: *mut PWSTR) {
    let service_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = RegisterServiceCtrlHandlerW(PCWSTR(service_name.as_ptr()), Some(service_handler));

    if let Ok(h) = handle {
        SERVICE_STATUS_HANDLE = Some(h);

        // Report START_PENDING
        let pending = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: SERVICE_START_PENDING,
            dwControlsAccepted: 0,
            dwWin32ExitCode: 0,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: 3000,
        };
        let _ = SetServiceStatus(h, &pending);

        // Report RUNNING
        let running = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: SERVICE_RUNNING,
            dwControlsAccepted: SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN,
            dwWin32ExitCode: 0,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: 0,
        };
        let _ = SetServiceStatus(h, &running);

        // Run the Named Pipe IPC server
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = pipe_ipc::run_server(true).await;
        });

        // Report STOPPED
        let stopped = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: SERVICE_STOPPED,
            dwControlsAccepted: 0,
            dwWin32ExitCode: 0,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: 0,
        };
        let _ = SetServiceStatus(h, &stopped);
    }
}

fn print_usage() {
    println!("VrV Desk Windows Service & UAC Elevation Daemon");
    println!("Usage: vrv_service [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --install        Register VrVDeskService with Windows Service Control Manager");
    println!("  --uninstall      Stop and remove VrVDeskService from SCM");
    println!("  --start          Start the VrVDeskService via SCM");
    println!("  --stop           Stop the VrVDeskService via SCM");
    println!("  --status         Check service installation, state, and elevation status");
    println!("  --daemon         Run IPC Named Pipe server in foreground daemon mode");
    println!("  --elevate [exe]  Request UAC elevation for current or specified executable");
    println!("  --sas            Trigger Secure Attention Sequence (Ctrl+Alt+Del)");
    println!("  --help           Show this help message");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("vrv_service.exe"));

    if args.len() > 1 {
        match args[1].as_str() {
            "--install" => {
                println!("[*] Installing VrV Desk Windows Service...");
                match install_service(&current_exe) {
                    Ok(_) => println!("[+] Service installed successfully!"),
                    Err(e) => eprintln!("[-] Failed to install service: {}", e),
                }
            }
            "--uninstall" => {
                println!("[*] Uninstalling VrV Desk Windows Service...");
                match uninstall_service() {
                    Ok(_) => println!("[+] Service uninstalled successfully!"),
                    Err(e) => eprintln!("[-] Failed to uninstall service: {}", e),
                }
            }
            "--start" => {
                println!("[*] Starting VrV Desk Windows Service...");
                match start_service() {
                    Ok(_) => println!("[+] Service started successfully!"),
                    Err(e) => eprintln!("[-] Failed to start service: {}", e),
                }
            }
            "--stop" => {
                println!("[*] Stopping VrV Desk Windows Service...");
                match stop_service() {
                    Ok(_) => println!("[+] Service stopped successfully!"),
                    Err(e) => eprintln!("[-] Failed to stop service: {}", e),
                }
            }
            "--status" => {
                let status = get_service_status();
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
            }
            "--daemon" => {
                println!("[*] Starting VrV Desk Named Pipe Server in foreground daemon mode...");
                println!("[*] Pipe: {}", SERVICE_PIPE_NAME);
                println!("[*] Elevated: {}", is_elevated());
                #[cfg(windows)]
                {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = pipe_ipc::run_server(false).await {
                            eprintln!("[-] Daemon error: {}", e);
                        }
                    });
                }
            }
            "--elevate" => {
                let target = args.get(2).map(|s| s.as_str());
                println!("[*] Requesting UAC elevation...");
                match request_elevation(target, None) {
                    Ok(_) => println!("[+] Elevation request dispatched successfully!"),
                    Err(e) => eprintln!("[-] Elevation failed: {}", e),
                }
            }
            "--sas" => {
                println!("[*] Triggering SAS (Ctrl+Alt+Del)...");
                match trigger_sas() {
                    Ok(_) => println!("[+] SAS triggered!"),
                    Err(e) => eprintln!("[-] SAS trigger failed: {}", e),
                }
            }
            "--help" | "-h" => {
                print_usage();
            }
            other => {
                eprintln!("Unknown option: {}", other);
                print_usage();
            }
        }
        return;
    }

    // Default mode: Attempt SCM dispatch
    #[cfg(windows)]
    unsafe {
        let service_name: Vec<u16> = SERVICE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        let service_table = [
            SERVICE_TABLE_ENTRYW {
                lpServiceName: PWSTR(service_name.as_ptr() as *mut _),
                lpServiceProc: Some(service_main),
            },
            SERVICE_TABLE_ENTRYW {
                lpServiceName: PWSTR(std::ptr::null_mut()),
                lpServiceProc: None,
            },
        ];

        // If called by SCM, this blocks and runs the service.
        // If run interactively from terminal/explorer, StartServiceCtrlDispatcherW returns error ERROR_FAILED_SERVICE_CONTROLLER_CONNECT (1063).
        if StartServiceCtrlDispatcherW(service_table.as_ptr()).is_err() {
            println!("VrV Desk Windows Service was launched interactively.");
            println!("To run as a daemon, use: vrv_service.exe --daemon");
            print_usage();
        }
    }
}
