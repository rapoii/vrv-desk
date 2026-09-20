use mirror_core::monitor::*;
use mirror_core::privacy::*;
use mirror_core::system_actions::*;

#[test]
fn test_enumerate_monitors_returns_at_least_one() {
    let monitors = enumerate_monitors();
    assert!(!monitors.is_empty(), "Monitors list should not be empty");
    let primary = monitors.iter().find(|m| m.is_primary);
    assert!(primary.is_some(), "At least one monitor should be primary");
    let first = &monitors[0];
    assert!(first.width > 0, "Monitor width should be positive");
    assert!(first.height > 0, "Monitor height should be positive");
    println!("Detected monitors: {:?}", monitors);
}

#[test]
fn test_privacy_mode_toggle() {
    assert!(!is_privacy_mode_enabled());
    assert!(set_privacy_mode(true).is_ok());
    assert!(is_privacy_mode_enabled());
    assert!(set_privacy_mode(false).is_ok());
    assert!(!is_privacy_mode_enabled());
}

#[test]
fn test_system_actions_validation() {
    // Valid actions validation (we don't want to actually reboot the host during unit tests!)
    let invalid = execute_system_action("invalid_action_xyz");
    assert!(invalid.is_err());

    // Task Manager or Abort Shutdown are safe to test
    let abort_res = execute_system_action("abort_shutdown");
    // Abort shutdown might succeed or fail depending on whether shutdown is scheduled, but shouldn't panic
    println!("Abort shutdown result: {:?}", abort_res);
}
