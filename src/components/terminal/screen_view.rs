use crate::pty::ScreenBuffer;
use dioxus::prelude::*;

#[component]
pub fn ScreenBufferView(screen: ScreenBuffer) -> Element {
    rsx! {
        pre { style: "margin: 0; white-space: pre; line-height: 1.2;",
            for (row_idx, row) in screen.grid.iter().enumerate() {
                div { key: "{row_idx}", style: "min-height: 1.2em; display: flex;",
                    for (col_idx, cell) in row.iter().enumerate() {
                        CellView {
                            ch: if cell.ch == '\0' { ' ' } else { cell.ch },
                            color: fg_color(cell.fg).to_string(),
                            bold: cell.bold,
                            cursor: row_idx == screen.cursor_row && col_idx == screen.cursor_col,
                            styled: cell.fg != 0 || cell.bold,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct CellViewProps {
    ch: char,
    color: String,
    bold: bool,
    cursor: bool,
    styled: bool,
}

#[component]
fn CellView(props: CellViewProps) -> Element {
    let bold = if props.bold { "font-weight: bold;" } else { "" };
    if props.cursor {
        return rsx! { span { style: "background: #6366f1; color: #0a0a14; {bold}", "{props.ch}" } };
    }
    if props.styled {
        return rsx! { span { style: "color: {props.color}; {bold}", "{props.ch}" } };
    }
    rsx! { span { "{props.ch}" } }
}

fn fg_color(code: u8) -> &'static str {
    match code {
        1 => "#ff5555",
        2 => "#50fa7b",
        3 => "#f1fa8c",
        4 => "#6366f1",
        5 => "#ff79c6",
        6 => "#8be9fd",
        7 => "#bbbbbb",
        8 => "#666666",
        9 => "#ff6e6e",
        10 => "#69ff94",
        11 => "#ffffa5",
        12 => "#8b8ff1",
        13 => "#ff92df",
        14 => "#a4ffff",
        15 => "#ffffff",
        _ => "#eaeaf0",
    }
}
