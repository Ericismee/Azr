use azr_core::{logo, settings::Settings};

#[test]
fn logo_choice_is_persisted_and_invalid_values_are_bounded() {
    assert_eq!(Settings::default().menu_logo, 1);
    for choice in 0..=2 {
        let s = Settings::parse(&format!("menu_logo={choice}"));
        assert_eq!(s.menu_logo, choice);
        assert_eq!(Settings::parse(&s.serialize()), s);
    }
    assert_eq!(Settings::parse("menu_logo=999").menu_logo, 2);
}

#[test]
fn small_brand_mark_has_an_apex_and_an_open_base() {
    let n = 100;
    let data = logo::render(n, [240, 240, 240]);
    let alpha = |x: u32, y: u32| data[((y * n + x) * 4 + 3) as usize];
    assert!(alpha(48, 16) > 200);
    assert_eq!(alpha(50, 85), 0);
    assert!(alpha(22, 84) > 200);
    assert_eq!(alpha(0, 0), 0);
    for p in data.chunks_exact(4) {
        assert!(p[..3].iter().all(|c| *c <= p[3]));
    }
}
