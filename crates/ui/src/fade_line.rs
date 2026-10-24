use std::panic::Location;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, ContentMask, ElementId, GlobalElementId, GridTemplate,
    GridTemplateMinSize, InspectorElementId, LayerFilter, LayoutId, Overflow, Pixels, Style,
    StyleRefinement, TextAlign, WhiteSpace, Window, div, px,
};

/// How far a line that runs out of room fades out toward its trailing edge.
const FADE: f32 = 28.;
/// How far a line may run past its room and still count as fitting, so that subpixel rounding
/// never sets a fade on a line that fits.
const SLACK: f32 = 0.5;

/// One line of text that never wraps and, where it runs out of room, fades out over its last
/// stretch instead of clipping hard or ending in an ellipsis. Whether it runs out is decided on
/// the frame it is laid out, so the fade is there from the first frame and gone the moment the
/// line fits. The fade is an alpha mask over the line alone, so it reads right over whatever
/// hover or selection fill lies beneath it.
pub struct FadeLine {
    style: StyleRefinement,
    line: Option<AnyElement>,
}

/// Lays `line` out as one fading line; see [`FadeLine`].
pub fn fade_line(line: impl IntoElement) -> FadeLine {
    FadeLine {
        style: StyleRefinement::default(),
        line: Some(line.into_any_element()),
    }
}

impl Styled for FadeLine {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for FadeLine {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl FadeLine {
    fn computed(&self) -> Style {
        // One grid column that may shrink to nothing lets the line count for its whole width
        // where its parent sizes to content, yet for none of it where the parent is squeezed,
        // so it never props open the row it sits in.
        let mut style = Style {
            display: gpui::Display::Grid,
            grid_cols: Some(GridTemplate {
                repeat: 1,
                min_size: GridTemplateMinSize::Zero,
            }),
            flex_shrink: 1.,
            ..Style::default()
        };
        style.min_size.width = px(0.).into();
        style.overflow.x = Overflow::Hidden;
        style.text.white_space = Some(WhiteSpace::Nowrap);
        style.refine(&self.style);
        style
    }
}

impl Element for FadeLine {
    type RequestLayoutState = Option<(AnyElement, LayoutId)>;
    type PrepaintState = bool;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = self.computed();
        let Some(line) = self.line.take() else {
            return (window.request_layout(style, [], cx), None);
        };
        let child = window.with_text_style(style.text_style().cloned(), |window| {
            // Auto margins take up only room to spare, so a centred or right-aligned line sits
            // where its alignment puts it while it fits and starts at the leading edge once it
            // runs over, fading out at the trailing one like any other.
            let align = window.text_style().text_align;
            let mut line = div()
                .flex_none()
                .map(|this| match align {
                    TextAlign::Left => this,
                    TextAlign::Center => this.mx_auto(),
                    TextAlign::Right => this.ml_auto(),
                })
                .child(line)
                .into_any_element();
            let child = line.request_layout(window, cx);
            let mut cell = Style {
                display: gpui::Display::Flex,
                ..Style::default()
            };
            cell.min_size.width = px(0.).into();
            let cell = window.request_layout(cell, [child], cx);
            (line, child, cell)
        });
        let (line, child, cell) = child;
        (
            window.request_layout(style, [cell], cx),
            Some((line, child)),
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        laid: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let Some((line, child)) = laid else {
            return false;
        };
        let runs_over = window.layout_bounds(*child).size.width > bounds.size.width + px(SLACK);
        let style = self.computed();
        window.with_text_style(style.text_style().cloned(), |window| {
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                line.prepaint(window, cx)
            })
        });
        runs_over
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        laid: &mut Self::RequestLayoutState,
        runs_over: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some((line, _)) = laid else {
            return;
        };
        let style = self.computed();
        let fade = match *runs_over {
            true => LayerFilter {
                fade_right: Some(px(FADE).min(bounds.size.width / 2.)),
                ..LayerFilter::default()
            },
            false => LayerFilter::default(),
        };
        window.with_text_style(style.text_style().cloned(), |window| {
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                window.with_filter(bounds, fade, Pixels::ZERO, |window| line.paint(window, cx))
            })
        });
    }
}
