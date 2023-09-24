use xterm_js_rs::xterm::Terminal;

use crate::terminal::WindowSize;

pub fn supports_keyboard_enhancement() -> std::io::Result<bool> {
    Ok(false)
}

pub fn window_size(terminal: &Terminal) -> std::io::Result<WindowSize> {
    let size = size(terminal).unwrap();

    Ok(WindowSize {
        rows: size.1,
        columns: size.0,
        width: 0,
        height: 0,
    })
}

pub fn size(terminal: &Terminal) -> std::io::Result<(u16, u16)> {
    Ok((
        terminal.get_cols().try_into().unwrap_or_else(|_| 0),
        terminal.get_rows().try_into().unwrap_or_else(|_| 0),
    ))
}

pub fn is_raw_mode_enabled() -> bool {
    true
}
