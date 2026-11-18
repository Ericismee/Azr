#[derive(Debug, Clone, Copy)]
pub struct RowLayout {
    pub label_y: f32,
    pub label_h: f32,
    pub control_x: f32,
    pub control_y: f32,
    pub control_w: f32,
    pub height: f32,
    pub stacked: bool,
}

pub fn row_layout(
    width: f32,
    label_width: f32,
    control_width: f32,
    label_height: f32,
    control_height: f32,
) -> RowLayout {
    let width = width.max(1.0);
    let cw = control_width.clamp(0.0, width);
    let stacked = label_width + cw + 16.0 > width;
    if stacked {
        RowLayout {
            label_y: 12.0,
            label_h: label_height,
            control_x: 0.0,
            control_y: 12.0 + label_height + 10.0,
            control_w: cw,
            height: 12.0 + label_height + 10.0 + control_height + 12.0,
            stacked,
        }
    } else {
        let height = (control_height.max(label_height) + 24.0).max(44.0);
        RowLayout {
            label_y: (height - label_height) / 2.0,
            label_h: label_height,
            control_x: width - cw,
            control_y: (height - control_height) / 2.0,
            control_w: cw,
            height,
            stacked,
        }
    }
}

pub fn wrap_words(text: &str, max_width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if measure(&candidate) <= max_width {
            line = candidate;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        for ch in word.chars() {
            let mut candidate = line.clone();
            candidate.push(ch);
            if !line.is_empty() && measure(&candidate) > max_width {
                lines.push(std::mem::take(&mut line));
            }
            line.push(ch);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[derive(Debug, Clone, Copy)]
pub struct ControlRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
pub fn flow_controls(width: f32, widths: &[f32], height: f32, gap: f32) -> Vec<ControlRect> {
    let (mut x, mut y) = (0.0, 0.0);
    let mut result = Vec::new();
    for w in widths {
        let w = w.clamp(1.0, width.max(1.0));
        if x > 0.0 && x + w > width {
            x = 0.0;
            y += height + gap;
        }
        result.push(ControlRect { x, y, w, h: height });
        x += w + gap;
    }
    result
}
