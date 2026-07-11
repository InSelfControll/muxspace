use super::*;

#[test]
fn screen_buffer_new() {
    let buf = ScreenBuffer::new(24, 80);
    assert_eq!(buf.rows, 24);
    assert_eq!(buf.cols, 80);
    assert_eq!(buf.grid.len(), 24);
    assert!(buf.title.is_empty());
}

#[test]
fn screen_buffer_process_simple_text() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"Hello");
    assert_eq!(buf.grid[0][0].ch, 'H');
    assert_eq!(buf.grid[0][4].ch, 'o');
    assert_eq!(buf.cursor_col, 5);
}

#[test]
fn screen_buffer_process_newline() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"Hello\nWorld");
    assert_eq!(buf.grid[1][0].ch, 'W');
    assert_eq!(buf.cursor_row, 1);
    assert_eq!(buf.cursor_col, 5);
}

#[test]
fn screen_buffer_process_carriage_return() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"Hello\rWorld");
    assert_eq!(buf.grid[0][0].ch, 'W');
    assert_eq!(buf.grid[0][4].ch, 'd');
}

#[test]
fn screen_buffer_cursor_movement() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"\x1b[5;10H");
    assert_eq!(buf.cursor_row, 4);
    assert_eq!(buf.cursor_col, 9);
}

#[test]
fn screen_buffer_scroll() {
    let mut buf = ScreenBuffer::new(3, 80);
    buf.process(b"Line1\nLine2\nLine3\nLine4");
    assert_eq!(buf.scrollback.len(), 1);
    assert_eq!(buf.grid.len(), 3);
}

#[test]
fn screen_buffer_tab_expansion() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"A\tB");
    assert_eq!(buf.cursor_col, 9);
    assert_eq!(buf.grid[0][8].ch, 'B');
}

#[test]
fn screen_buffer_backspace() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"Hello\x08X");
    assert_eq!(buf.grid[0][4].ch, 'X');
}

#[test]
fn screen_buffer_erase_display() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"Hello World");
    buf.process(b"\x1b[6;1H\x1b[J");
    assert_eq!(buf.grid[5][0].ch, '\0');
}

#[test]
fn screen_buffer_osc_title() {
    let mut buf = ScreenBuffer::new(24, 80);
    buf.process(b"\x1b]2;My Terminal Title\x1b\\");
    assert_eq!(buf.title, "My Terminal Title");
}

#[test]
fn screen_buffer_partial_eq_and_clone() {
    let buf1 = ScreenBuffer::new(24, 80);
    let mut buf2 = buf1.clone();
    assert_eq!(buf1, buf2);
    buf2.process(b"Hello");
    assert_ne!(buf1, buf2);
}

#[test]
fn pty_manager_new() {
    let mgr = PtyManager::new();
    assert!(mgr.is_empty());
    assert!(mgr.active_pane_ids().is_empty());
}
