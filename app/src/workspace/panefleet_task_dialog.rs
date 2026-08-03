//! Names the work an environment is for.
//!
//! One field, because a task line reads the way people already write them:
//! `SEC-1802 Onboard Miro audit logs`. The tracker key is split off on submit
//! (see `PaneFleetTaskBinding::from_input`) rather than asked for separately.
//! Template, gate and schedule arrive with the full "New task" sheet in P1.

use pathfinder_geometry::vector::vec2f;
use warp_core::ui::theme::Fill;
use warpui::elements::{
    Align, ChildAnchor, ChildView, Clipped, ConstrainedBox, Container, CornerRadius,
    CrossAxisAlignment, Element, Flex, MainAxisSize, OffsetPositioning, ParentAnchor,
    ParentElement, ParentOffsetBounds, Radius, Stack, Text,
};
use warpui::keymap::{FixedBinding, Keystroke};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{
    AppContext, Entity, FocusContext, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use std::path::PathBuf;

use super::panefleet_tasks::PaneFleetTaskBinding;
use crate::appearance::Appearance;
use crate::editor::{
    EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys, SingleLineEditorOptions,
    TextOptions,
};
use crate::ui_components::dialog::{Dialog, dialog_styles};
use crate::view_components::action_button::{
    ActionButton, KeystrokeSource, NakedTheme, PrimaryTheme,
};

pub(super) fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        PaneFleetTaskDialogAction::Cancel,
        id!(PaneFleetTaskDialog::ui_name()),
    )]);
}

const DIALOG_WIDTH: f32 = 520.;
const INPUT_PADDING: f32 = 8.;
const INPUT_FONT_SIZE: f32 = 13.;

#[derive(Clone)]
pub(super) struct PaneFleetTaskDialogSource {
    pub environment_path: PathBuf,
    /// The binding being edited, or `None` when naming the work for the first time.
    pub existing: Option<PaneFleetTaskBinding>,
    /// This row is a project, so confirming creates a folder under it for the
    /// work rather than renaming the project itself. Said out loud because it
    /// writes to disk.
    pub creates_directory: bool,
}

pub(super) struct PaneFleetTaskDialog {
    editor: ViewHandle<EditorView>,
    done_check_editor: ViewHandle<EditorView>,
    cancel_button: ViewHandle<ActionButton>,
    save_button: ViewHandle<ActionButton>,
    source: Option<PaneFleetTaskDialogSource>,
}

impl PaneFleetTaskDialog {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let editor = ctx.add_typed_action_view(|ctx| {
            let options = {
                let appearance = Appearance::as_ref(ctx);
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(INPUT_FONT_SIZE), appearance),
                    select_all_on_focus: true,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    ..Default::default()
                }
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("SEC-1802 Onboard Miro audit logs", ctx);
            editor
        });
        ctx.subscribe_to_view(&editor, |me, _, event, ctx| match event {
            EditorEvent::Enter => me.confirm(ctx),
            EditorEvent::Escape => ctx.emit(PaneFleetTaskDialogEvent::Cancel),
            _ => ctx.notify(),
        });

        let done_check_editor = ctx.add_typed_action_view(|ctx| {
            let options = {
                let appearance = Appearance::as_ref(ctx);
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(INPUT_FONT_SIZE), appearance),
                    select_all_on_focus: true,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    ..Default::default()
                }
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("cargo test  —  leave empty for no gate", ctx);
            editor
        });
        ctx.subscribe_to_view(&done_check_editor, |me, _, event, ctx| match event {
            EditorEvent::Enter => me.confirm(ctx),
            EditorEvent::Escape => ctx.emit(PaneFleetTaskDialogEvent::Cancel),
            _ => ctx.notify(),
        });

        let cancel_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Cancel", NakedTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(PaneFleetTaskDialogAction::Cancel);
            })
        });
        let enter_keystroke = Keystroke::parse("enter").expect("Valid keystroke");
        let save_button = ctx.add_typed_action_view(|ctx| {
            ActionButton::new("Set Task", PrimaryTheme)
                .with_keybinding(KeystrokeSource::Fixed(enter_keystroke), ctx)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(PaneFleetTaskDialogAction::Confirm);
                })
        });

        Self {
            editor,
            done_check_editor,
            cancel_button,
            save_button,
            source: None,
        }
    }

    pub fn set_source(&mut self, source: PaneFleetTaskDialogSource, ctx: &mut ViewContext<Self>) {
        let text = source
            .existing
            .as_ref()
            .map(PaneFleetTaskBinding::input_text)
            .unwrap_or_default();
        let label = if source.existing.is_some() {
            "Rename Task"
        } else {
            "Set Task"
        };
        self.save_button.update(ctx, |button, ctx| {
            button.set_label(label.to_string(), ctx);
        });
        let done_check = source
            .existing
            .as_ref()
            .map(PaneFleetTaskBinding::done_check_text)
            .unwrap_or_default();
        self.editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(&text, ctx);
        });
        self.done_check_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(&done_check, ctx);
        });
        self.source = Some(source);
        ctx.notify();
    }

    fn confirm(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        let input = self.editor.as_ref(ctx).buffer_text(ctx);
        let done_check = self.done_check_editor.as_ref(ctx).buffer_text(ctx);
        ctx.emit(PaneFleetTaskDialogEvent::Confirm(Box::new(
            PaneFleetTaskSubmission {
                environment_path: source.environment_path,
                existing: source.existing,
                input,
                done_check,
            },
        )));
    }

    fn render_fields(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(6.)
            .with_child(self.render_label("TASK", appearance))
            .with_child(self.render_editor(&self.editor, app))
            .with_child(self.render_label("DONE WHEN", appearance))
            .with_child(self.render_editor(&self.done_check_editor, app))
            .finish()
    }

    /// Two identical-looking inputs are indistinguishable without these.
    fn render_label(&self, text: &'static str, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        Text::new_inline(text.to_string(), appearance.ui_font_family(), 10.)
            .with_color(theme.sub_text_color(theme.background()).into())
            .finish()
    }

    fn render_editor(&self, editor: &ViewHandle<EditorView>, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let height = editor.as_ref(app).line_height(app.font_cache(), appearance);
        let editor = ConstrainedBox::new(Clipped::new(ChildView::new(editor).finish()).finish())
            .with_height(height)
            .finish();
        Container::new(editor)
            .with_uniform_padding(INPUT_PADDING)
            .with_background(theme.background())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .finish()
    }
}

impl Entity for PaneFleetTaskDialog {
    type Event = PaneFleetTaskDialogEvent;
}

impl View for PaneFleetTaskDialog {
    fn ui_name() -> &'static str {
        "PaneFleetTaskDialog"
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            ctx.focus(&self.editor);
            ctx.notify();
        }
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let Some(source) = &self.source else {
            return Container::new(Stack::new().finish()).finish();
        };

        let title = if source.existing.is_some() {
            "Rename task".to_string()
        } else {
            "What is this environment for?".to_string()
        };
        let body = if source.creates_directory {
            format!(
                "This is a project, not work done in it. Confirming creates a folder for the task \
                 inside it and opens that as an environment — the way a worktree appears. A leading \
                 tracker key such as SEC-1802 names the folder.\n\n{}",
                source.environment_path.display()
            )
        } else {
            format!(
                "The task names the sidebar row and the tabs of this environment; the branch moves \
                 to secondary metadata. A leading tracker key such as SEC-1802 or inc-36884 is \
                 recognized and shown separately.\n\n{}",
                source.environment_path.display()
            )
        };

        let cancel_button = Container::new(ChildView::new(&self.cancel_button).finish())
            .with_margin_right(12.)
            .finish();
        let dialog = Dialog::new(
            title,
            Some(body),
            UiComponentStyles {
                width: Some(DIALOG_WIDTH),
                ..dialog_styles(appearance)
            },
        )
        // One child, not two: `Dialog::with_child` assigns rather than appends,
        // so a second call silently replaces the first field.
        .with_child(self.render_fields(app))
        .with_bottom_row_child(cancel_button)
        .with_bottom_row_child(ChildView::new(&self.save_button).finish())
        .build()
        .finish();

        let mut stack = Stack::new();
        stack.add_positioned_child(
            dialog,
            OffsetPositioning::offset_from_parent(
                vec2f(0., 0.),
                ParentOffsetBounds::WindowByPosition,
                ParentAnchor::Center,
                ChildAnchor::Center,
            ),
        );
        Container::new(Align::new(stack.finish()).finish())
            .with_background_color(Fill::blur().into())
            .with_corner_radius(app.windows().window_corner_radius())
            .finish()
    }
}

pub(super) struct PaneFleetTaskSubmission {
    pub environment_path: PathBuf,
    pub existing: Option<PaneFleetTaskBinding>,
    pub input: String,
    /// Shell command line deciding whether the work is done. Empty means no
    /// gate, and then the task never reaches `Done` on its own.
    pub done_check: String,
}

pub(super) enum PaneFleetTaskDialogEvent {
    /// Boxed so the variant carrying a whole binding does not set the size of
    /// every event, including the empty `Cancel`.
    Confirm(Box<PaneFleetTaskSubmission>),
    Cancel,
}

#[derive(Debug)]
pub(super) enum PaneFleetTaskDialogAction {
    Confirm,
    Cancel,
}

impl TypedActionView for PaneFleetTaskDialog {
    type Action = PaneFleetTaskDialogAction;

    fn handle_action(&mut self, action: &PaneFleetTaskDialogAction, ctx: &mut ViewContext<Self>) {
        match action {
            PaneFleetTaskDialogAction::Confirm => self.confirm(ctx),
            PaneFleetTaskDialogAction::Cancel => {
                ctx.emit(PaneFleetTaskDialogEvent::Cancel);
            }
        }
    }
}
