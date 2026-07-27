use pathfinder_geometry::vector::vec2f;
use warp_core::ui::theme::Fill;
use warp_errors::report_error;
use warpui::elements::{
    Align, ChildAnchor, ChildView, Container, OffsetPositioning, ParentAnchor, ParentOffsetBounds,
    Stack,
};
use warpui::keymap::{FixedBinding, Keystroke};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::panefleet_worktrees::PaneFleetWorktreeRemovalInspection;
use crate::appearance::Appearance;
use crate::ui_components::dialog::{Dialog, dialog_styles};
use crate::view_components::action_button::{
    ActionButton, DangerPrimaryTheme, KeystrokeSource, NakedTheme,
};

pub(super) fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([
        FixedBinding::new(
            "escape",
            PaneFleetWorktreeRemovalDialogAction::Cancel,
            id!(PaneFleetWorktreeRemovalDialog::ui_name()),
        ),
        FixedBinding::new(
            "enter",
            PaneFleetWorktreeRemovalDialogAction::Confirm,
            id!(PaneFleetWorktreeRemovalDialog::ui_name()),
        ),
    ]);
}

const DIALOG_WIDTH: f32 = 520.;

#[derive(Clone)]
pub(super) struct PaneFleetWorktreeRemovalDialogSource {
    pub inspection: PaneFleetWorktreeRemovalInspection,
    pub delete_branch: bool,
}

pub(super) struct PaneFleetWorktreeRemovalDialog {
    cancel_button: ViewHandle<ActionButton>,
    remove_button: ViewHandle<ActionButton>,
    source: Option<PaneFleetWorktreeRemovalDialogSource>,
}

impl PaneFleetWorktreeRemovalDialog {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let cancel_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Cancel", NakedTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(PaneFleetWorktreeRemovalDialogAction::Cancel);
            })
        });
        let enter_keystroke = Keystroke::parse("enter").expect("Valid keystroke");
        let remove_button = ctx.add_typed_action_view(|ctx| {
            ActionButton::new("Remove Worktree", DangerPrimaryTheme)
                .with_keybinding(KeystrokeSource::Fixed(enter_keystroke), ctx)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(PaneFleetWorktreeRemovalDialogAction::Confirm);
                })
        });

        Self {
            cancel_button,
            remove_button,
            source: None,
        }
    }

    pub fn set_source(
        &mut self,
        source: PaneFleetWorktreeRemovalDialogSource,
        ctx: &mut ViewContext<Self>,
    ) {
        let label = if source.delete_branch {
            "Remove Worktree & Branch"
        } else {
            "Remove Worktree"
        };
        self.remove_button.update(ctx, |button, ctx| {
            button.set_label(label.to_string(), ctx);
        });
        self.source = Some(source);
        ctx.notify();
    }
}

impl Entity for PaneFleetWorktreeRemovalDialog {
    type Event = PaneFleetWorktreeRemovalDialogEvent;
}

impl View for PaneFleetWorktreeRemovalDialog {
    fn ui_name() -> &'static str {
        "PaneFleetWorktreeRemovalDialog"
    }

    fn on_focus(&mut self, _focus_ctx: &warpui::FocusContext, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let Some(source) = &self.source else {
            return Container::new(Stack::new().finish()).finish();
        };
        let inspection = &source.inspection;
        let title = if source.delete_branch {
            format!("Remove worktree and branch '{}'?", inspection.branch)
        } else {
            format!("Remove worktree '{}'?", inspection.branch)
        };
        let upstream = match &inspection.upstream {
            Some(upstream) if inspection.unpushed_commit_count > 0 => format!(
                "Upstream: {upstream} ({} unpushed commit{})",
                inspection.unpushed_commit_count,
                if inspection.unpushed_commit_count == 1 {
                    ""
                } else {
                    "s"
                }
            ),
            Some(upstream) => format!("Upstream: {upstream}"),
            None => "No upstream branch is configured".to_string(),
        };
        let branch_effect = if source.delete_branch {
            "Git will remove the folder, then safely delete the local branch. If the branch is not merged, Git will keep it."
        } else {
            "Git will remove the folder and worktree registration. The local branch will be kept."
        };
        let info = format!(
            "Path: {}\nBranch: {}\n{}\n\nAll tabs and agent processes in this environment will be closed. {}",
            inspection.path.display(),
            inspection.branch,
            upstream,
            branch_effect
        );
        let cancel_button = Container::new(ChildView::new(&self.cancel_button).finish())
            .with_margin_right(12.)
            .finish();
        let dialog = Dialog::new(
            title,
            Some(info),
            UiComponentStyles {
                width: Some(DIALOG_WIDTH),
                ..dialog_styles(appearance)
            },
        )
        .with_bottom_row_child(cancel_button)
        .with_bottom_row_child(ChildView::new(&self.remove_button).finish())
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

pub(super) enum PaneFleetWorktreeRemovalDialogEvent {
    Confirm {
        source: PaneFleetWorktreeRemovalDialogSource,
    },
    Cancel,
}

#[derive(Debug)]
pub(super) enum PaneFleetWorktreeRemovalDialogAction {
    Confirm,
    Cancel,
}

impl TypedActionView for PaneFleetWorktreeRemovalDialog {
    type Action = PaneFleetWorktreeRemovalDialogAction;

    fn handle_action(
        &mut self,
        action: &PaneFleetWorktreeRemovalDialogAction,
        ctx: &mut ViewContext<Self>,
    ) {
        match action {
            PaneFleetWorktreeRemovalDialogAction::Confirm => {
                let Some(source) = self.source.clone() else {
                    report_error!("Remove worktree confirm pressed with no source");
                    return;
                };
                ctx.emit(PaneFleetWorktreeRemovalDialogEvent::Confirm { source });
            }
            PaneFleetWorktreeRemovalDialogAction::Cancel => {
                ctx.emit(PaneFleetWorktreeRemovalDialogEvent::Cancel);
            }
        }
    }
}
