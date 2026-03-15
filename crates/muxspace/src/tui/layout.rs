use ratatui::layout::Rect;

/// Computed area for a single pane.
pub struct PaneArea {
    pub index: usize,
    pub rect: Rect,
}

/// Divide the available area evenly among `n` vertical panes.
pub fn compute_pane_areas(area: Rect, n: usize) -> Vec<PaneArea> {
    if n == 0 {
        return vec![];
    }
    let n = n as u16;
    let base_width = area.width / n;
    let remainder = area.width % n;

    let mut areas = Vec::with_capacity(n as usize);
    let mut current_x = area.x;

    for i in 0..n {
        let extra = if (i as u16) < remainder { 1 } else { 0 };
        let width = base_width + extra;
        areas.push(PaneArea {
            index: i as usize,
            rect: Rect {
                x: current_x,
                y: area.y,
                width,
                height: area.height,
            },
        });
        current_x += width;
    }
    areas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_pane_areas_even() {
        let area = Rect::new(0, 0, 100, 50);
        let areas = compute_pane_areas(area, 4);
        assert_eq!(areas.len(), 4);
        assert_eq!(areas[0].rect.width, 25);
        assert_eq!(areas[0].rect.x, 0);
        assert_eq!(areas[1].rect.x, 25);
    }

    #[test]
    fn test_compute_pane_areas_uneven() {
        let area = Rect::new(0, 0, 100, 50);
        let areas = compute_pane_areas(area, 3);
        assert_eq!(areas.len(), 3);
        // 100 / 3 = 33 remainder 1
        assert_eq!(areas[0].rect.width, 34); // gets the extra
        assert_eq!(areas[1].rect.width, 33);
        assert_eq!(areas[2].rect.width, 33);
    }

    #[test]
    fn test_compute_pane_areas_zero() {
        let area = Rect::new(0, 0, 100, 50);
        let areas = compute_pane_areas(area, 0);
        assert!(areas.is_empty());
    }
}
