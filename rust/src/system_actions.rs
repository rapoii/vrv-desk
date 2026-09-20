//! Remote System Actions (Lock, Task Manager, Reboot, Shutdown)
//! Provides execution of essential administrative and session commands safely.

pub fn execute_system_action(action: &str) -> Result<String, String> {
    match action {
        "lock" => {
            #[cfg(windows)]
            unsafe {
                use windows::Win32::System::Shutdown::LockWorkStation;
                LockWorkStation().map_err(|e| format!("LockWorkStation failed: {:?}", e))?;
            }
            Ok("Workstation locked successfully".to_string())
        }
        "taskmgr" => {
            #[cfg(windows)]
            {
                std::process::Command::new("taskmgr.exe")
                    .spawn()
                    .map_err(|e| format!("Failed to spawn Task Manager: {:?}", e))?;
            }
            Ok("Task Manager launched".to_string())
        }
        "reboot" => {
            #[cfg(windows)]
            {
                std::process::Command::new("shutdown.exe")
                    .args(["/r", "/t", "5", "/c", "VrV Desk Remote Reboot"])
                    .spawn()
                    .map_err(|e| format!("Failed to schedule reboot: {:?}", e))?;
            }
            Ok("System reboot scheduled in 5 seconds".to_string())
        }
        "shutdown" => {
            #[cfg(windows)]
            {
                std::process::Command::new("shutdown.exe")
                    .args(["/s", "/t", "10", "/c", "VrV Desk Remote Shutdown"])
                    .spawn()
                    .map_err(|e| format!("Failed to schedule shutdown: {:?}", e))?;
            }
            Ok("System shutdown scheduled in 10 seconds".to_string())
        }
        "abort_shutdown" => {
            #[cfg(windows)]
            {
                std::process::Command::new("shutdown.exe")
                    .arg("/a")
                    .spawn()
                    .map_err(|e| format!("Failed to abort shutdown: {:?}", e))?;
            }
            Ok("System shutdown aborted".to_string())
        }
        other => Err(format!("Unknown system action: {}", other)),
    }
}
