use azr_core::settings_layout::{row_layout, wrap_words};

#[test]
fn long_labels_stack_instead_of_overlapping_controls() {
    let row = row_layout(480.0, 440.0, 300.0, 18.0, 28.0);
    assert!(row.stacked);
    assert!(row.control_y >= row.label_y + row.label_h + 8.0);
    assert!(row.control_x >= 0.0 && row.control_x + row.control_w <= 480.0);
}
#[test]
fn fitting_rows_keep_a_clear_horizontal_gap() {
    let row = row_layout(480.0, 90.0, 200.0, 18.0, 28.0);
    assert!(!row.stacked);
    assert!(row.control_x >= 90.0 + 16.0);
    assert!(row.height >= 44.0);
}
#[test]
fn vietnamese_captions_wrap_to_their_own_cells() {
    let lines = wrap_words("Biểu tượng trong suốt", 72.0, |s| {
        s.chars().count() as f32 * 7.0
    });
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|s| s.chars().count() <= 10));
    assert_eq!(lines.join(" "), "Biểu tượng trong suốt");
}
#[test]
fn long_unbroken_text_is_split_at_unicode_boundaries() {
    let lines = wrap_words("TiếngViệtRấtDài", 28.0, |s| {
        s.chars().count() as f32 * 7.0
    });
    assert!(lines.iter().all(|s| s.chars().count() <= 4));
    assert_eq!(lines.concat(), "TiếngViệtRấtDài");
}

#[test]
fn wide_controls_flow_without_crossing_the_card_boundary() {
    let boxes = azr_core::settings_layout::flow_controls(240.0, &[100.0, 180.0, 80.0], 28.0, 8.0);
    assert!(boxes[1].y > boxes[0].y);
    assert!(boxes.iter().all(|b| b.x >= 0.0 && b.x + b.w <= 240.0));
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            assert!(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y);
        }
    }
}
