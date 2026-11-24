use azr_core::dock::layout::{layout, Metrics, Slot};
use azr_core::i18n::Lang;
use azr_core::settings::{Behavior, Settings, Theme};
use azr_core::widgets::{self, Kind};

#[test]
fn fresh_settings_match_the_desktop_preset_without_changing_legacy_defaults() {
    let fresh = Settings::first_launch();
    assert_eq!(fresh.lang, Lang::En);
    assert_eq!(fresh.theme, Theme::Dark);
    assert_eq!(fresh.dock, Behavior::Smart);
    assert_eq!(
        (fresh.dock_size, fresh.dock_opacity, fresh.dock_hide_delay),
        (38, 19, 3)
    );
    assert_eq!(
        (fresh.widget_style, fresh.launchpad_icon, fresh.main_display),
        (1, 1, 0)
    );
    assert!(fresh.takeover && fresh.notch_enabled && fresh.overview_alt_tab);
    assert!(!fresh.quick_power_actions && !fresh.clipboard_history && !fresh.autostart);
    assert_eq!(Settings::parse(&fresh.serialize()), fresh);
    assert_eq!(Settings::parse("dock_size=62").dock_size, 62);
    assert!(!Settings::parse("dock_size=62").takeover);
}

#[test]
fn default_widgets_are_visible_and_separated_at_different_display_sizes() {
    for (screen, dpi) in [
        ((800, 600), 2.0),
        ((1024, 768), 1.5),
        ((1366, 768), 1.25),
        ((1914, 1075), 1.0),
        ((3840, 2160), 2.0),
    ] {
        let origin = (-1200, 80);
        let dpi = widgets::fit_dpi(screen, dpi);
        let area = widgets::desktop_area(origin, screen, dpi);
        let placed = widgets::default_layout(area, dpi);
        assert_eq!(
            placed.map(|p| p.kind),
            [Kind::Calendar, Kind::Clock, Kind::Battery, Kind::System]
        );
        let size = widgets::size_of(Kind::Clock, dpi).0.ceil() as i32;
        for (i, p) in placed.iter().enumerate() {
            assert!(p.x >= area.0 && p.y >= area.1);
            assert!(p.x + size <= area.0 + area.2 && p.y + size <= area.1 + area.3);
            for q in &placed[i + 1..] {
                assert!((p.x - q.x).abs() >= size || (p.y - q.y).abs() >= size);
            }
        }
    }
}

#[test]
fn fitted_dock_keeps_even_magnified_icons_inside_the_screen() {
    let slots = [Slot::Item; 18];
    let metrics = Metrics::custom(38.0, 6.0, 1.24, 2.0);
    let fitted = metrics.fit_width(&slots, 720.0);
    assert!(fitted.icon < metrics.icon);
    assert_eq!(fitted.max_scale, metrics.max_scale);
    assert_eq!(metrics.fit_width(&slots, 3840.0), metrics);
    for cursor in (0..720).step_by(11) {
        let positions = layout(&slots, Some(cursor as f32), 720.0, &fitted);
        assert!(positions.total <= 720.0);
    }
}
