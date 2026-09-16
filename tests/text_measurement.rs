//! The single measurement path. Before this module there were eight call
//! sites with three different constants, all counting UTF-8 bytes.

use agent_illustrator::layout::text::{
    advance_em, measure_runs, measure_str, RichText, TextRun, LINE_HEIGHT_FACTOR,
};

#[test]
fn ascii_advances_come_from_the_table_not_a_flat_constant() {
    // A lowercase 'l' is narrow and an 'M' is wide; a flat per-character
    // constant cannot tell them apart, which is how boxes ended up
    // mis-sized for both short and long words.
    assert!(
        advance_em('l') < 0.30,
        "'l' should be narrow, got {}",
        advance_em('l')
    );
    assert!(
        advance_em('M') > 0.75,
        "'M' should be wide, got {}",
        advance_em('M')
    );
    assert!(advance_em(' ') > 0.0, "space must have width");
}

#[test]
fn multibyte_characters_count_as_one_character() {
    // "≤" is 3 bytes. The old `str::len()` sites measured it as three
    // characters, inflating every box holding a maths symbol or em dash.
    let one_le = measure_str("≤", 14.0);
    let three_ascii = measure_str("mmm", 14.0);
    assert!(
        one_le < three_ascii,
        "a single ≤ ({one_le}) must not measure wider than three m's ({three_ascii})"
    );
}

#[test]
fn unlisted_characters_fall_back_rather_than_measuring_zero() {
    // A CJK or emoji character has no table entry; zero width would let it
    // silently overflow its box.
    assert!(advance_em('漢') > 0.0);
}

#[test]
fn measuring_a_string_scales_linearly_with_font_size() {
    let at_14 = measure_str("hello world", 14.0);
    let at_28 = measure_str("hello world", 28.0);
    assert!(
        (at_28 - at_14 * 2.0).abs() < 0.001,
        "{at_14} vs {at_28}"
    );
}

#[test]
fn runs_measure_to_the_widest_line_and_stack_in_height() {
    let rich = RichText {
        lines: vec![
            vec![TextRun::plain("short")],
            vec![TextRun::plain("a much longer line")],
        ],
    };
    let m = measure_runs(&rich.lines, 14.0);
    assert_eq!(m.line_count, 2);
    assert!((m.width - measure_str("a much longer line", 14.0)).abs() < 0.001);
    assert!((m.height - 2.0 * 14.0 * LINE_HEIGHT_FACTOR).abs() < 0.001);
}

#[test]
fn bold_runs_measure_wider_than_normal_ones() {
    let normal = measure_runs(&[vec![TextRun::plain("changeset")]], 14.0);
    let bold = measure_runs(
        &[vec![TextRun {
            bold: true,
            ..TextRun::plain("changeset")
        }]],
        14.0,
    );
    assert!(
        bold.width > normal.width,
        "{} vs {}",
        bold.width,
        normal.width
    );
}

#[test]
fn small_runs_measure_narrower_and_do_not_change_line_height() {
    let normal = measure_runs(&[vec![TextRun::plain("inherits")]], 14.0);
    let small = measure_runs(
        &[vec![TextRun {
            scale: 0.8,
            ..TextRun::plain("inherits")
        }]],
        14.0,
    );
    assert!(small.width < normal.width);
    assert!((small.height - normal.height).abs() < 0.001);
}

use agent_illustrator::{render_with_lint, RenderConfig};

fn render_str(source: &str) -> String {
    agent_illustrator::render(source).expect("renders")
}

#[test]
fn a_label_that_fits_its_box_is_not_reported_as_overflowing() {
    // The box was sized at 8.0px/char and checked at 8.4px/char, so labels
    // that fit were reported as straddling. One measurer, one answer.
    let source = r#"rect card [label: "temporal_mode = correction"]"#;
    let (_svg, warnings) =
        render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    assert!(
        warnings.is_empty(),
        "an auto-sized box must fit its own label, got: {:?}",
        warnings.iter().map(|w| w.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn a_label_full_of_maths_symbols_does_not_inflate_its_box() {
    // Each ≤ is 3 UTF-8 bytes; the old sites measured it as 3 characters.
    let narrow = render_str(r#"rect a [label: "T <= now"]"#);
    let symbolic = render_str(r#"rect a [label: "T ≤ now"]"#);
    let width_of = |svg: &str| -> f64 {
        let at = svg.find("width=\"").expect("a width attribute");
        svg[at + 7..]
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap_or(0.0)
    };
    assert!(
        width_of(&symbolic) <= width_of(&narrow) + 1.0,
        "≤ should not measure wider than <=: {} vs {}",
        width_of(&symbolic),
        width_of(&narrow)
    );
}
