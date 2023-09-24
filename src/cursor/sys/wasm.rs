use xterm_js_rs::Terminal;

pub fn position(terminal: &Terminal) -> std::io::Result<(u16, u16)> {
    Ok((
        terminal
            .get_buffer()
            .get_active()
            .get_cursor_x()
            .try_into()
            .unwrap_or_else(|_| 0),
        terminal
            .get_buffer()
            .get_active()
            .get_cursor_y()
            .try_into()
            .unwrap_or_else(|_| 0),
    ))
}
