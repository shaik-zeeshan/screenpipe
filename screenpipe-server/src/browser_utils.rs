use std::process::Command;

#[cfg(target_os = "macos")]
pub fn get_browser_url(app_name: &str) -> Option<String> {
    let is_browser = matches!(
        app_name.to_lowercase().as_str(),
        "safari" | "google chrome" | "arc" | "brave browser"
    );

    if !is_browser {
        return None;
    }

    let script = match app_name.to_lowercase().as_str() {
        "safari" => r#"tell application "Safari" to get URL of current tab of front window"#,
        "google chrome" => {
            r#"tell application "Google Chrome" to get URL of active tab of front window"#
        }
        "arc" => r#"tell application "Arc" to get URL of active tab of front window"#,
        "brave browser" => {
            r#"tell application "Brave Browser" to get URL of active tab of front window"#
        }
        _ => return None,
    };

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn get_browser_url(app_name: &str) -> Option<String> {
    use std::path::Path;
    use windows::Win32::System::ProcessStatus::GetProcessImageFileNameW;
    use windows::Win32::UI::Accessibility::{
        IUIAutomation, TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_EditControlTypeId,
        UIA_NamePropertyId,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId,
    };

    let is_browser = matches!(
        app_name.to_lowercase().as_str(),
        "chrome" | "firefox" | "msedge" | "brave" | "opera"
    );

    if !is_browser {
        return None;
    }

    unsafe {
        let automation = windows::Win32::UI::Accessibility::CoCreateInstance::<_, IUIAutomation>(
            &windows::Win32::UI::Accessibility::CUIAutomation::default(),
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
        .ok()?;

        // Store target window handle
        let mut target_hwnd = None;

        EnumWindows(
            Some(|hwnd, lparam| -> i32 {
                let mut process_id = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut process_id));

                // Get process name
                let process_handle =
                    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id);

                let mut buffer = [0u16; 260];
                if GetProcessImageFileNameW(process_handle, &mut buffer) > 0 {
                    let process_name = String::from_utf16_lossy(&buffer)
                        .trim_matches(char::from(0))
                        .to_lowercase();

                    if Path::new(&process_name)
                        .file_name()
                        .and_then(|f| f.to_str())
                        .map(|s| s.contains(&app_name.to_lowercase()))
                        .unwrap_or(false)
                    {
                        target_hwnd = Some(hwnd);
                        return 0; // Stop enumeration
                    }
                }

                1 // Continue enumeration
            }),
            None,
        );

        let hwnd = target_hwnd?;
        let element = automation.ElementFromHandle(hwnd).ok()?;

        // Find address bar
        let condition = automation
            .CreatePropertyCondition(UIA_ControlTypePropertyId, UIA_EditControlTypeId.into())
            .ok()?;

        let address_bar = element.FindFirst(TreeScope_Descendants, &condition).ok()?;

        let pattern = address_bar
            .GetCurrentPattern(windows::Win32::UI::Accessibility::UIA_ValuePatternPropertyId)
            .ok()?;

        let value_pattern: windows::Win32::UI::Accessibility::IUIAutomationValuePattern =
            pattern.cast().ok()?;

        let url = value_pattern.CurrentValue().ok()?;
        Some(url.to_string())
    }
}

#[cfg(target_os = "linux")]
pub fn get_browser_url(app_name: &str) -> Option<String> {
    use gio::prelude::*;
    use gio::{BusType, DBusConnection};

    let is_browser = matches!(
        app_name.to_lowercase().as_str(),
        "firefox" | "chromium" | "google-chrome" | "brave-browser"
    );

    if !is_browser {
        return None;
    }

    let connection =
        DBusConnection::new_for_bus_sync(BusType::Session, gio::NONE_CANCELLABLE).ok()?;

    match app_name.to_lowercase().as_str() {
        "firefox" => {
            let msg = gio::DBusMessage::new_method_call(
                Some("org.mozilla.firefox"),
                "/org/mozilla/firefox",
                "org.mozilla.firefox",
                "GetCurrentURL",
            );

            let reply = connection
                .send_message_with_reply_sync(&msg, gio::NONE_CANCELLABLE)
                .ok()?;

            let variant = reply.body().get::<String>().ok()?;
            Some(variant)
        }
        "chromium" | "google-chrome" | "brave-browser" => {
            let browser_name = match app_name.to_lowercase().as_str() {
                "chromium" => "org.chromium",
                "google-chrome" => "com.google.Chrome",
                "brave-browser" => "com.brave.Browser",
                _ => return None,
            };

            let msg = gio::DBusMessage::new_method_call(
                Some(browser_name),
                "/org/chromium/Browser",
                "org.chromium.Browser",
                "GetCurrentTabURL",
            );

            let reply = connection
                .send_message_with_reply_sync(&msg, gio::NONE_CANCELLABLE)
                .ok()?;

            let variant = reply.body().get::<String>().ok()?;
            Some(variant)
        }
        _ => None,
    }
}
