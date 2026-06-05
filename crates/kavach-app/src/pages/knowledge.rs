mod data;

use dioxus::prelude::*;

use crate::state::{REFRESH_TICK, SELECTED_PROJECT};
use data::{CANVAS_H, CANVAS_W, LoadState, color_for, load};

#[component]
pub(crate) fn KnowledgePage() -> Element {
    let proj = SELECTED_PROJECT.read().clone();
    let tick: u64 = *REFRESH_TICK.read();
    let filter = use_signal(|| None::<String>);
    let Some(slug) = proj else {
        return rsx! {
            section { class: "page page-knowledge",
                div { class: "graph-header",
                    h1 { "Knowledge Graph" }
                    div { class: "graph-stats", "—" }
                }
                p { class: "hint", "Select a project from the sidebar." }
            }
        };
    };

    let etype = filter.read().clone();
    // The force layout is O(iters * n^2); memoize so it runs once per
    // (slug, tick, filter) change, not on every re-render.
    let payload = use_resource(use_reactive!(|(slug, tick, etype)| async move {
        let _ = tick;
        load(&slug, etype.as_deref())
    }));
    let snap = payload.read_unchecked();
    let (shown, total) = match &*snap {
        Some(LoadState::Ok { shown, total, .. }) => (*shown, *total),
        Some(LoadState::DaemonOffline | LoadState::Empty | LoadState::Failed(_)) | None => (0, 0),
    };

    let mut view = use_signal(Viewport::default);
    let vb = view.read().view_box();

    rsx! {
        section { class: "page page-knowledge",
            div { class: "graph-header",
                h1 { "Concept Graph" }
                div { class: "graph-stats", "{shown} shown of {total} entities" }
            }
            div { class: "graph-filters",
                {FILTERS.iter().map(|&(value, label)| filter_chip(filter, value, label))}
            }
            div { class: "graph-controls",
                button { class: "graph-chip", onclick: move |_| zoom(view, ZOOM_STEP), "+" }
                button { class: "graph-chip", onclick: move |_| zoom(view, 1.0 / ZOOM_STEP), "−" }
                button { class: "graph-chip", onclick: move |_| pan(view, 0.0, -PAN_STEP), "↑" }
                button { class: "graph-chip", onclick: move |_| pan(view, 0.0, PAN_STEP), "↓" }
                button { class: "graph-chip", onclick: move |_| pan(view, -PAN_STEP, 0.0), "←" }
                button { class: "graph-chip", onclick: move |_| pan(view, PAN_STEP, 0.0), "→" }
                button { class: "graph-chip", onclick: move |_| view.set(Viewport::default()), "reset" }
            }
            match &*snap {
                None => rsx! { div { class: "skeleton", "Computing layout…" } },
                Some(LoadState::DaemonOffline) => rsx! {
                    div { class: "banner banner-warn",
                        "kavach-rpc daemon offline — start via "
                        code { "kavach rpc serve" }
                    }
                },
                Some(LoadState::Empty) => rsx! { div { class: "empty", "No graph entities for this filter." } },
                Some(LoadState::Failed(msg)) => rsx! {
                    div { class: "banner banner-error",
                        "Graph load failed: "
                        code { "{msg}" }
                    }
                },
                Some(LoadState::Ok { nodes, edges, .. }) => rsx! {
                    svg {
                        class: "graph-canvas",
                        view_box: "{vb}",
                        width: "100%",
                        // Edges first so nodes paint on top of them.
                        for e in edges.iter() {
                            line {
                                x1: "{e.x1}", y1: "{e.y1}", x2: "{e.x2}", y2: "{e.y2}",
                                stroke: "#3a3a4a", stroke_width: "1",
                            }
                        }
                        for n in nodes.iter() {
                            g { transform: "translate({n.x},{n.y})",
                                title { "{n.label} · {n.kind}" }
                                circle { r: "6", fill: "{color_for(&n.kind)}", stroke: "#1a1a22", stroke_width: "1" }
                                text {
                                    x: "9", y: "3", fill: "#cfd2dc", font_size: "10",
                                    "{truncate_label(&n.label)}"
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

/// The entity-type filter chips, in display order. `None` = show all kinds.
const FILTERS: &[(Option<&str>, &str)] = &[
    (None, "all"),
    (Some("concept"), "concept"),
    (Some("skill"), "skill"),
    (Some("file"), "file"),
    (Some("decision"), "decision"),
];

/// Multiplicative zoom per `+`/`−` press, and pan distance (in canvas units)
/// per arrow press. Pan scales with the current zoom so it feels constant
/// on-screen regardless of how far in the user is.
const ZOOM_STEP: f64 = 1.25;
const PAN_STEP: f64 = 0.12;
const MIN_SCALE: f64 = 0.2;
const MAX_SCALE: f64 = 6.0;

/// The SVG viewport: a scale (zoom) and a top-left offset (pan), rendered into
/// an SVG `view_box`. Smaller width/height = more zoomed in. State lives in a
/// signal so controls mutate it and the `<svg>` re-renders.
///
/// `scale` is private and only set through [`Viewport::new`], which clamps it to
/// [`MIN_SCALE`, `MAX_SCALE`] — so a `Viewport` can never hold an out-of-range
/// scale that would divide the canvas into an invisible or jittering view.
#[derive(Clone, Copy, PartialEq)]
struct Viewport {
    scale: f64,
    x: f64,
    y: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }
}

impl Viewport {
    /// Construct a viewport, clamping `scale` into the valid zoom range. This is
    /// the only constructor, so the scale invariant holds for every `Viewport`.
    const fn new(scale: f64, x: f64, y: f64) -> Self {
        Self {
            scale: scale.clamp(MIN_SCALE, MAX_SCALE),
            x,
            y,
        }
    }

    /// `"minX minY width height"` — narrower span at higher scale zooms in.
    #[expect(
        clippy::float_arithmetic,
        reason = "viewport span is scale division; float math is intrinsic to SVG coordinates"
    )]
    fn view_box(self) -> String {
        let w = CANVAS_W / self.scale;
        let h = CANVAS_H / self.scale;
        format!("{} {} {w} {h}", self.x, self.y)
    }
}

/// Zoom about the canvas center by `factor`, clamped to [`MIN_SCALE`,
/// `MAX_SCALE`], keeping the visible center fixed so zoom doesn't drift.
#[expect(
    clippy::float_arithmetic,
    reason = "zoom/pan are continuous viewport transforms; float math is intrinsic"
)]
fn zoom(mut view: Signal<Viewport>, factor: f64) {
    let v = *view.read();
    let new_scale = (v.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
    if (new_scale - v.scale).abs() < f64::EPSILON {
        return;
    }
    let (old_w, old_h) = (CANVAS_W / v.scale, CANVAS_H / v.scale);
    let (new_w, new_h) = (CANVAS_W / new_scale, CANVAS_H / new_scale);
    // Hold the center point: shift the origin by half the span delta.
    let x = (old_w - new_w).mul_add(0.5, v.x);
    let y = (old_h - new_h).mul_add(0.5, v.y);
    view.set(Viewport::new(new_scale, x, y));
}

/// Pan by a fraction of the *visible* span, so movement feels constant
/// regardless of zoom level.
#[expect(
    clippy::float_arithmetic,
    reason = "pan offset is a fraction of the visible span; float math is intrinsic"
)]
fn pan(mut view: Signal<Viewport>, frac_x: f64, frac_y: f64) {
    let v = *view.read();
    let x = (CANVAS_W / v.scale).mul_add(frac_x, v.x);
    let y = (CANVAS_H / v.scale).mul_add(frac_y, v.y);
    view.set(Viewport::new(v.scale, x, y));
}

/// Render one filter chip. A plain fn (not a `#[component]`) so the page keeps a
/// single derived `Props`, avoiding the `same_name_method` collision Dioxus's
/// macro emits when two components share a module.
fn filter_chip(
    mut current: Signal<Option<String>>,
    value: Option<&'static str>,
    label: &'static str,
) -> Element {
    let owned = value.map(str::to_owned);
    let active = *current.read() == owned;
    rsx! {
        button {
            class: if active { "graph-chip graph-chip-active" } else { "graph-chip" },
            onclick: move |_| current.set(owned.clone()),
            "{label}"
        }
    }
}

/// Keep node labels short enough that the canvas stays readable; long entity
/// names (file paths, skill keys) would otherwise overlap badly.
fn truncate_label(s: &str) -> String {
    const MAX: usize = 24;
    if s.chars().count() <= MAX {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(MAX - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::{MAX_SCALE, MIN_SCALE, Viewport, truncate_label};

    #[test]
    fn viewport_new_clamps_scale_into_range() {
        // The whole point of making scale private behind `new`: out-of-range
        // input can never be stored, even via a direct construction.
        assert!((Viewport::new(1000.0, 0.0, 0.0).scale - MAX_SCALE).abs() < f64::EPSILON);
        assert!((Viewport::new(-5.0, 0.0, 0.0).scale - MIN_SCALE).abs() < f64::EPSILON);
        let mid = Viewport::new(2.0, 0.0, 0.0);
        assert!(
            (mid.scale - 2.0).abs() < f64::EPSILON,
            "in-range scale preserved"
        );
    }

    #[test]
    fn default_viewport_is_neutral() {
        let v = Viewport::default();
        assert!((v.scale - 1.0).abs() < f64::EPSILON && v.x == 0.0 && v.y == 0.0);
    }

    #[test]
    fn truncate_label_boundaries() {
        assert_eq!(truncate_label(""), "", "empty stays empty");
        let exactly = "a".repeat(24);
        assert_eq!(truncate_label(&exactly), exactly, "24 chars unchanged");
        let over = "b".repeat(25);
        let out = truncate_label(&over);
        assert_eq!(out.chars().count(), 24, "25 chars -> 23 + ellipsis = 24");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn truncate_label_counts_chars_not_bytes() {
        // Multi-byte chars must not be split mid-codepoint; "é" is 1 char, 2 bytes.
        let s = "é".repeat(30);
        let out = truncate_label(&s);
        assert_eq!(out.chars().count(), 24, "char-based, not byte-based");
        assert!(out.is_char_boundary(out.len()), "no corrupted UTF-8");
    }
}
