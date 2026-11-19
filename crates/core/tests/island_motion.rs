use azr_core::notch::state::motion;

// Simulate a unit spring step: visible expansion should be progressive, settle
// within half a second, and avoid a pronounced rubber-band overshoot.
fn response(t_ms: f32, ratio: f32, period_ms: u32) -> f32 {
    let omega = std::f32::consts::TAU / period_ms as f32;
    let t = t_ms;
    if ratio >= 1.0 {
        return 1.0 - (1.0 + omega * t) * (-omega * t).exp();
    }
    let d = (1.0 - ratio * ratio).sqrt();
    1.0 - (-ratio * omega * t).exp() * ((omega * d * t).cos() + ratio / d * (omega * d * t).sin())
}

#[test]
fn island_expansion_has_visible_travel_without_large_bounce() {
    let at_first_frames = response(32.0, motion::OPEN_RATIO, motion::OPEN_PERIOD_MS);
    assert!(
        (0.1..0.65).contains(&at_first_frames),
        "opening jumps too far: {at_first_frames}"
    );
    for t in 0..500 {
        assert!(response(t as f32, motion::OPEN_RATIO, motion::OPEN_PERIOD_MS) <= 1.025);
    }
    assert!((response(450.0, motion::OPEN_RATIO, motion::OPEN_PERIOD_MS) - 1.0).abs() < 0.005);
}
