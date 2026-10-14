use super::{Counts, verify_dense_terminal};
use anyhow::Result;

fn dense_redraws(redraws: usize) -> Counts {
    Counts {
        quads: 50 * redraws,
        glyphs: 6981 * redraws,
        decorations: 1921 * redraws,
        paints: crate::terminal_painter::Layer::ALL.len() * redraws,
        ..Default::default()
    }
}

#[test]
fn dense_budgets_count_complete_layer_groups() -> Result<()> {
    verify_dense_terminal(dense_redraws(1), true)?;
    verify_dense_terminal(dense_redraws(2), true)?;
    Ok(())
}

#[test]
fn dense_budgets_reject_missing_or_incomplete_redraws() {
    for paints in [0, 1, 2, 4, 5] {
        let counts = Counts {
            paints,
            ..dense_redraws(1)
        };
        assert!(verify_dense_terminal(counts, true).is_err(), "{counts:?}");
    }
}

#[test]
fn dense_budgets_remain_exact_for_every_layer() {
    let expected = dense_redraws(1);
    for counts in [
        Counts {
            quads: 49,
            ..expected
        },
        Counts {
            quads: 51,
            ..expected
        },
        Counts {
            glyphs: 6980,
            ..expected
        },
        Counts {
            glyphs: 6982,
            ..expected
        },
        Counts {
            decorations: 1920,
            ..expected
        },
        Counts {
            decorations: 1922,
            ..expected
        },
        Counts {
            paint_errors: 1,
            ..expected
        },
        Counts {
            shapes: 401,
            ..expected
        },
    ] {
        assert!(verify_dense_terminal(counts, false).is_err(), "{counts:?}");
    }
}

#[test]
fn warm_dense_redraws_never_shape_text_or_metrics() -> Result<()> {
    let expected = dense_redraws(1);
    verify_dense_terminal(
        Counts {
            shapes: 140,
            ..expected
        },
        false,
    )?;
    for counts in [
        Counts {
            shapes: 1,
            ..expected
        },
        Counts {
            metric_shapes: 1,
            ..expected
        },
    ] {
        assert!(verify_dense_terminal(counts, true).is_err(), "{counts:?}");
    }
    Ok(())
}
