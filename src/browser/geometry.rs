#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn normalize_browser_bounds(
    viewport_width: f64,
    window_width: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<BrowserBounds> {
    let values = [viewport_width, window_width, x, y, width, height];
    if values.iter().any(|value| !value.is_finite()) || viewport_width.abs() < 1.0 {
        return None;
    }

    let broken_webkit_scale = viewport_width.is_sign_negative();
    let scale = if broken_webkit_scale {
        window_width.abs() / viewport_width.abs()
    } else {
        1.0
    };
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }

    let coordinate = |value: f64| {
        let value = if broken_webkit_scale {
            value.abs()
        } else {
            value
        };
        (value * scale).round() as i32
    };
    let size = |value: f64| (value.abs() * scale).round() as i32;
    let bounds = BrowserBounds {
        x: coordinate(x),
        y: coordinate(y),
        width: size(width),
        height: size(height),
    };

    (bounds.width > 20 && bounds.height > 20).then_some(bounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_normal_webkit_bounds() {
        let bounds = normalize_browser_bounds(1400.0, 1400.0, 260.0, 90.0, 900.0, 600.0);
        assert_eq!(
            bounds,
            Some(BrowserBounds {
                x: 260,
                y: 90,
                width: 900,
                height: 600,
            })
        );
    }

    #[test]
    fn repairs_negative_high_dpi_webkit_bounds() {
        let bounds = normalize_browser_bounds(
            -134_400.0, 1400.0, -24_000.0, -8_640.0, -96_000.0, -57_600.0,
        );
        assert_eq!(
            bounds,
            Some(BrowserBounds {
                x: 250,
                y: 90,
                width: 1000,
                height: 600,
            })
        );
    }

    #[test]
    fn rejects_missing_or_tiny_bounds() {
        assert_eq!(
            normalize_browser_bounds(0.0, 1400.0, 0.0, 0.0, 900.0, 600.0),
            None
        );
        assert_eq!(
            normalize_browser_bounds(1400.0, 1400.0, 0.0, 0.0, 10.0, 10.0),
            None
        );
    }
}
