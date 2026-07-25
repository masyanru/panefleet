use warp_core::features::FeatureFlag;
use warpui::elements::{
    Container, CrossAxisAlignment, Flex, MainAxisAlignment, MainAxisSize, ParentElement, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::ui_components::components::UiComponent;
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{AppContext, Element, Entity, TypedActionView, View, ViewContext, ViewHandle};

use super::SettingsSection;
use super::settings_page::{
    MatchData, PageType, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
};
use crate::appearance::Appearance;
use crate::workspace::panefleet_preferences::PaneFleetWorkspacePreferences;

#[derive(Debug, Clone)]
pub enum PaneFleetWorkspaceSettingsPageEvent {
    PreferencesChanged,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneFleetWorkspaceSettingsPageAction {
    ToggleWorkspacePath,
    ToggleGitBranch,
    ToggleAgentActivity,
}

pub struct PaneFleetWorkspaceSettingsPageView {
    page: PageType<Self>,
    preferences: PaneFleetWorkspacePreferences,
    workspace_path_switch: SwitchStateHandle,
    git_branch_switch: SwitchStateHandle,
    agent_activity_switch: SwitchStateHandle,
}

impl PaneFleetWorkspaceSettingsPageView {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        let path = PaneFleetWorkspacePreferences::path();
        let preferences = PaneFleetWorkspacePreferences::load_or_default(&path);
        if !path.exists()
            && let Err(error) = preferences.write_atomic(&path)
        {
            log::warn!("Failed to write PaneFleet workspace preferences: {error}");
        }

        Self {
            page: PageType::new_monolith(PaneFleetWorkspaceSettingsWidget, None, true),
            preferences,
            workspace_path_switch: SwitchStateHandle::default(),
            git_branch_switch: SwitchStateHandle::default(),
            agent_activity_switch: SwitchStateHandle::default(),
        }
    }

    fn persist(&self, ctx: &mut ViewContext<Self>) {
        if let Err(error) = self
            .preferences
            .write_atomic(&PaneFleetWorkspacePreferences::path())
        {
            log::warn!("Failed to write PaneFleet workspace preferences: {error}");
            return;
        }
        ctx.emit(PaneFleetWorkspaceSettingsPageEvent::PreferencesChanged);
        ctx.notify();
    }

    fn render_toggle_row(
        &self,
        title: &'static str,
        description: &'static str,
        checked: bool,
        switch_state: SwitchStateHandle,
        action: PaneFleetWorkspaceSettingsPageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        Container::new(
            Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Flex::column()
                        .with_main_axis_size(MainAxisSize::Min)
                        .with_child(
                            Text::new_inline(
                                title,
                                appearance.ui_font_family(),
                                appearance.ui_font_size(),
                            )
                            .with_style(Properties::default().weight(Weight::Medium))
                            .with_color(theme.active_ui_text_color().into())
                            .finish(),
                        )
                        .with_child(
                            Text::new_inline(
                                description,
                                appearance.ui_font_family(),
                                appearance.ui_font_size() - 1.,
                            )
                            .with_color(theme.sub_text_color(theme.background()).into())
                            .finish(),
                        )
                        .finish(),
                )
                .with_child(
                    appearance
                        .ui_builder()
                        .switch(switch_state)
                        .check(checked)
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(action.clone());
                        })
                        .finish(),
                )
                .finish(),
        )
        .with_padding_top(12.)
        .with_padding_bottom(12.)
        .finish()
    }
}

impl Entity for PaneFleetWorkspaceSettingsPageView {
    type Event = PaneFleetWorkspaceSettingsPageEvent;
}

impl View for PaneFleetWorkspaceSettingsPageView {
    fn ui_name() -> &'static str {
        "PaneFleetWorkspaceSettingsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl TypedActionView for PaneFleetWorkspaceSettingsPageView {
    type Action = PaneFleetWorkspaceSettingsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            PaneFleetWorkspaceSettingsPageAction::ToggleWorkspacePath => {
                self.preferences.show_workspace_path = !self.preferences.show_workspace_path;
            }
            PaneFleetWorkspaceSettingsPageAction::ToggleGitBranch => {
                self.preferences.show_git_branch = !self.preferences.show_git_branch;
            }
            PaneFleetWorkspaceSettingsPageAction::ToggleAgentActivity => {
                self.preferences.show_agent_activity = !self.preferences.show_agent_activity;
            }
        }
        self.persist(ctx);
    }
}

impl SettingsPageMeta for PaneFleetWorkspaceSettingsPageView {
    fn section() -> SettingsSection {
        SettingsSection::PaneFleetWorkspace
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        FeatureFlag::PaneFleetWorkbench.is_enabled()
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<PaneFleetWorkspaceSettingsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<PaneFleetWorkspaceSettingsPageView>) -> Self {
        SettingsPageViewHandle::PaneFleetWorkspace(view_handle)
    }
}

struct PaneFleetWorkspaceSettingsWidget;

impl SettingsWidget for PaneFleetWorkspaceSettingsWidget {
    type View = PaneFleetWorkspaceSettingsPageView;

    fn search_terms(&self) -> &str {
        "workspace projects sidebar path working directory git branch activity agents"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_child(
                Text::new_inline("Workspace", appearance.ui_font_family(), 23.)
                    .with_style(Properties::default().weight(Weight::Bold))
                    .with_color(theme.active_ui_text_color().into())
                    .finish(),
            )
            .with_child(
                Container::new(
                    Text::new_inline(
                        "Choose which live project details PaneFleet shows in the Workspaces sidebar.",
                        appearance.ui_font_family(),
                        appearance.ui_font_size(),
                    )
                    .with_color(theme.sub_text_color(theme.background()).into())
                    .finish(),
                )
                .with_margin_top(6.)
                .with_margin_bottom(18.)
                .finish(),
            )
            .with_child(view.render_toggle_row(
                "Show workspace path",
                "Display the full workspace root below its name.",
                view.preferences.show_workspace_path,
                view.workspace_path_switch.clone(),
                PaneFleetWorkspaceSettingsPageAction::ToggleWorkspacePath,
                appearance,
            ))
            .with_child(view.render_toggle_row(
                "Show Git branch",
                "Display the current branch or detached commit next to the workspace path.",
                view.preferences.show_git_branch,
                view.git_branch_switch.clone(),
                PaneFleetWorkspaceSettingsPageAction::ToggleGitBranch,
                appearance,
            ))
            .with_child(view.render_toggle_row(
                "Show agent activity",
                "Animate workspace indicators only while an agent is actively working.",
                view.preferences.show_agent_activity,
                view.agent_activity_switch.clone(),
                PaneFleetWorkspaceSettingsPageAction::ToggleAgentActivity,
                appearance,
            ))
            .finish()
    }
}
