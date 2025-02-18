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
    use windows::Win32::UI::Accessibility::{IUIAutomation, IUIAutomationElement};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let is_browser = matches!(
        app_name.to_lowercase().as_str(),
        "chrome" | "firefox" | "msedge" | "brave" | "opera"
    );

    if !is_browser {
        return None;
    }

    // Get the foreground window handle
    let hwnd = unsafe { GetForegroundWindow() };

    // Initialize UI Automation
    let automation = IUIAutomation::create().ok()?;
    let element = unsafe { automation.ElementFromHandle(hwnd).ok()? };

    // Get the address bar element (varies by browser)
    let address_bar = element
        .FindFirst(
            TreeScope::Descendants,
            automation.CreatePropertyCondition(UIA_ControlTypePropertyId, UIA_EditControlTypeId)?,
        )
        .ok()?;

    // Get the URL from the address bar
    let url_value = address_bar
        .GetCurrentPropertyValue(UIA_ValuePatternPropertyId)
        .ok()?;
    url_value.GetString().ok()
}

#[cfg(target_os = "linux")]
pub fn get_browser_url(app_name: &str) -> Option<String> {
    let is_browser = matches!(
        app_name.to_lowercase().as_str(),
        "chromium" | "firefox" | "google-chrome" | "brave-browser"
    );

    if !is_browser {
        return None;
    }

    // Using xdotool to get active window info and then dbus to get URL
    let window_id = Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()?;

    if !window_id.status.success() {
        return None;
    }

    let browser_name = match app_name.to_lowercase().as_str() {
        "firefox" => "org.mozilla.firefox",
        "chromium" | "google-chrome" => "org.chromium.Chromium",
        "brave-browser" => "com.brave.Browser",
        _ => return None,
    };

    // Using dbus-send to get URL from browser
    let output = Command::new("dbus-send")
        .args(&[
            "--session",
            "--dest=".to_owned() + browser_name,
            "--type=method_call",
            "--print-reply",
            "/org/mozilla/firefox/Window",
            "org.mozilla.firefox.Window.GetURL",
        ])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .and_then(|s| s.lines().last().map(String::from))
    } else {
        None
    }
}
