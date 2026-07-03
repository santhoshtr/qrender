//! Icon symbols (Material Symbols, Apache-2.0), ported from the wx
//! prototype. Each asset file is a ready-made <symbol> element; the
//! factoid page embeds only the symbols it uses (tree-shaken sprite).

/// Sorted by name for binary search.
static ICONS: &[(&str, &str)] = &[
    ("3d_rotation", include_str!("../assets/icons/3d_rotation.svg")),
    ("altitude", include_str!("../assets/icons/altitude.svg")),
    ("archive", include_str!("../assets/icons/archive.svg")),
    ("arrow_cool_down", include_str!("../assets/icons/arrow_cool_down.svg")),
    ("article", include_str!("../assets/icons/article.svg")),
    ("aspect_ratio", include_str!("../assets/icons/aspect_ratio.svg")),
    ("bloodtype", include_str!("../assets/icons/bloodtype.svg")),
    ("bold", include_str!("../assets/icons/bold.svg")),
    ("calendar_month", include_str!("../assets/icons/calendar_month.svg")),
    ("call", include_str!("../assets/icons/call.svg")),
    ("captive_portal", include_str!("../assets/icons/captive_portal.svg")),
    ("communities", include_str!("../assets/icons/communities.svg")),
    ("compare_arrows", include_str!("../assets/icons/compare_arrows.svg")),
    ("directions_car", include_str!("../assets/icons/directions_car.svg")),
    ("distance", include_str!("../assets/icons/distance.svg")),
    ("equal", include_str!("../assets/icons/equal.svg")),
    ("event", include_str!("../assets/icons/event.svg")),
    ("fit_width", include_str!("../assets/icons/fit_width.svg")),
    ("globe", include_str!("../assets/icons/globe.svg")),
    ("group", include_str!("../assets/icons/group.svg")),
    ("groups", include_str!("../assets/icons/groups.svg")),
    ("hand", include_str!("../assets/icons/hand.svg")),
    ("height", include_str!("../assets/icons/height.svg")),
    ("home", include_str!("../assets/icons/home.svg")),
    ("how_to_vote", include_str!("../assets/icons/how_to_vote.svg")),
    ("info", include_str!("../assets/icons/info.svg")),
    ("language", include_str!("../assets/icons/language.svg")),
    ("local_library", include_str!("../assets/icons/local_library.svg")),
    ("location_on", include_str!("../assets/icons/location_on.svg")),
    ("man", include_str!("../assets/icons/man.svg")),
    ("map", include_str!("../assets/icons/map.svg")),
    ("mountain_flag", include_str!("../assets/icons/mountain_flag.svg")),
    ("open_in_phone", include_str!("../assets/icons/open_in_phone.svg")),
    ("pace", include_str!("../assets/icons/pace.svg")),
    ("person", include_str!("../assets/icons/person.svg")),
    ("photo_library", include_str!("../assets/icons/photo_library.svg")),
    ("policy", include_str!("../assets/icons/policy.svg")),
    ("school", include_str!("../assets/icons/school.svg")),
    ("sell", include_str!("../assets/icons/sell.svg")),
    ("stethoscope", include_str!("../assets/icons/stethoscope.svg")),
    ("straighten", include_str!("../assets/icons/straighten.svg")),
    ("tag", include_str!("../assets/icons/tag.svg")),
    ("thermometer", include_str!("../assets/icons/thermometer.svg")),
    ("thermostat_arrow_down", include_str!("../assets/icons/thermostat_arrow_down.svg")),
    ("thermostat_arrow_up", include_str!("../assets/icons/thermostat_arrow_up.svg")),
    ("train", include_str!("../assets/icons/train.svg")),
    ("trophy", include_str!("../assets/icons/trophy.svg")),
    ("video_library", include_str!("../assets/icons/video_library.svg")),
    ("visibility", include_str!("../assets/icons/visibility.svg")),
    ("volume_up", include_str!("../assets/icons/volume_up.svg")),
    ("weight", include_str!("../assets/icons/weight.svg")),
    ("woman", include_str!("../assets/icons/woman.svg")),
];

pub fn lookup(name: &str) -> Option<&'static str> {
    ICONS
        .binary_search_by_key(&name, |(n, _)| n)
        .ok()
        .map(|i| ICONS[i].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_for_binary_search() {
        assert!(ICONS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn symbols_are_well_formed_and_fill_free() {
        for (name, svg) in ICONS {
            assert!(svg.starts_with(&format!("<symbol id=\"i-{name}\"")), "{name}");
            assert!(svg.trim_end().ends_with("</symbol>"), "{name}");
            assert!(!svg.contains("fill="), "{name} must inherit currentColor");
        }
    }
}
