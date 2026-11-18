use azr_core::{
    iconplate,
    settings::{self, Settings},
};

#[test]
fn dock_appearance_round_trips_and_clamps_untrusted_config() {
    let s = Settings::parse(
        "dock_surface=1\ndock_opacity=28\ndock_blur=57\nicon_round=72\nicon_background=1\n",
    );
    assert_eq!(
        (
            s.dock_surface,
            s.dock_opacity,
            s.dock_blur,
            s.icon_round,
            s.icon_background
        ),
        (1, 28, 57, 72, 1)
    );
    assert_eq!(Settings::parse(&s.serialize()), s);
    let s = Settings::parse(
        "dock_surface=999\ndock_opacity=-9\ndock_blur=999\nicon_round=999\nicon_background=999",
    );
    assert_eq!(
        (
            s.dock_surface,
            s.dock_opacity,
            s.dock_blur,
            s.icon_round,
            s.icon_background
        ),
        (3, 0, 100, 100, 2)
    );
}

#[test]
fn opacity_changes_fallback_alpha_and_clear_surface_removes_fill() {
    let mut s = Settings::default();
    s.dock_opacity = 20;
    let a = settings::dock_look(&s, false);
    s.dock_opacity = 80;
    let b = settings::dock_look(&s, false);
    assert!(a.solid[0] < b.solid[0]);
    assert!(a.tint[0] < b.tint[0]);
    assert!(a.liquify);
    s.dock_surface = 2;
    let clear = settings::dock_look(&s, false);
    assert_eq!((clear.solid[0], clear.tint[0]), (0, 0));
    assert!(!clear.liquify);
    s.dock_surface = 3;
    assert_eq!(settings::dock_look(&s, false).solid[0], 255);
}

#[test]
fn continuous_icon_corners_keep_more_area_than_a_circle() {
    let n = 64;
    let opaque = vec![255; (n * n * 4) as usize];
    let square = iconplate::round_icon(&opaque, n, 0);
    let circle = iconplate::round_icon(&opaque, n, 75);
    let a = |image: &[u8], x: u32, y: u32| image[((y * n + x) * 4 + 3) as usize];
    assert!(a(&square, 8, 8) > 200);
    assert!(a(&circle, 8, 8) > 150);
    assert_eq!(a(&circle, 32, 32), 255);
    assert_eq!(a(&circle, 4, 4), 0);
    assert_eq!(
        iconplate::round_icon(&opaque, n, 255),
        iconplate::round_icon(&opaque, n, 100)
    );
}

#[test]
fn translucent_icon_plate_keeps_premultiplied_alpha() {
    let n = 64;
    let mut raw = vec![0; (n * n * 4) as usize];
    for y in 24..40 {
        for x in 24..40 {
            let i = ((y * n + x) * 4) as usize;
            raw[i..i + 4].copy_from_slice(&[10, 30, 220, 255]);
        }
    }
    let glass = iconplate::plate_custom(&raw, n, 45, 1);
    assert!(glass.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 180));
    for p in glass.chunks_exact(4) {
        assert!(p[..3].iter().all(|c| *c <= p[3]));
    }
    assert_eq!(iconplate::round_icon(&[], 0, 0), Vec::<u8>::new());
}
