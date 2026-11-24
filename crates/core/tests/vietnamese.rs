use azr_core::{
    i18n::{self, Lang},
    settings::Settings,
};

#[test]
fn vietnamese_translation_remains_available_and_english_is_default() {
    let vi = Lang::parse("vi").expect("Vietnamese locale");
    assert_eq!(Settings::default().lang, Lang::En);
    assert_eq!(Lang::ALL[0], Lang::En);
    assert_eq!(vi.key(), "vi");
    assert_eq!(Lang::parse("id"), None);
    assert_eq!(i18n::tr(vi, "Settings"), "Cài đặt");
    assert_eq!(i18n::tr(Lang::En, "Settings"), "Settings");
    assert_eq!(i18n::month_name(vi, 11, "November"), "Tháng 11");
    assert_eq!(i18n::weekday_labels(vi, true, [""; 7])[0], "T2");
    assert_eq!(
        azr_core::notch::glance::date_label(2026, 11, 19, vi),
        "T5, 19 Th11"
    );
    assert_eq!(
        azr_core::notch::glance::date_label(2026, 12, 19, vi),
        "T7, 19 Th12"
    );
    assert_eq!(
        azr_core::notch::glance::date_label(2026, 11, 19, Lang::En),
        "Thu, 19 Nov"
    );
}
