use mirror_core::platform::windows_input::{
    get_clipboard_text, inject_shortcut, inject_unicode_text, set_clipboard_text,
};

#[test]
fn test_inject_unicode_text() {
    let res = inject_unicode_text("Hello World! 🚀");
    assert!(res.is_ok());
}

#[test]
fn test_inject_shortcuts() {
    let shortcuts = [
        "win",
        "esc",
        "enter",
        "backspace",
        "tab",
        "task_manager",
        "alt_tab",
        "show_desktop",
    ];

    for s in shortcuts {
        let res = inject_shortcut(s);
        assert!(res.is_ok(), "Shortcut '{}' failed", s);
    }

    let unknown = inject_shortcut("invalid_shortcut_name");
    assert!(unknown.is_err(), "Unknown shortcut should return error");
}

#[test]
fn test_clipboard_operations() {
    let test_str = "vrv_desk_clipboard_unit_test_12345";
    let ok = set_clipboard_text(test_str);
    assert!(ok, "set_clipboard_text should succeed");

    let read_back = get_clipboard_text();
    assert_eq!(
        read_back,
        Some(test_str.to_string()),
        "get_clipboard_text should match set text"
    );
}
