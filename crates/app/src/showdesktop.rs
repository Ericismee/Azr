/// Use Windows' native Show Desktop toggle so this stays in sync with Win+D,
/// taskbar actions, and other applications.
pub fn toggle() {
    azr_platform::keys::chord(&[azr_platform::keys::WIN, 0x44]);
}
