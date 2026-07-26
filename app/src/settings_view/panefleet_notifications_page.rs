use warp_core::features::FeatureFlag;
use warpui::elements::{
    ChildView, Container, CrossAxisAlignment, Flex, MainAxisAlignment, MainAxisSize, ParentElement,
    Text,
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
use crate::ui_components::icons::Icon;
use crate::view_components::Dropdown;
use crate::view_components::action_button::{ActionButton, SecondaryTheme};
use crate::view_components::dropdown::DropdownItem;
use crate::workspace::panefleet_notifications::{
    PaneFleetNotificationPreferences, PaneFleetNotificationSound,
};

const SOUND_DROPDOWN_WIDTH: f32 = 180.;

#[derive(Debug, Clone, PartialEq)]
pub enum PaneFleetNotificationsSettingsPageAction {
    ToggleAgentCompletionSound,
    SetAgentCompletionSound(PaneFleetNotificationSound),
    Preview,
}

pub struct PaneFleetNotificationsSettingsPageView {
    page: PageType<Self>,
    preferences: PaneFleetNotificationPreferences,
    enabled_switch: SwitchStateHandle,
    sound_dropdown: ViewHandle<Dropdown<PaneFleetNotificationsSettingsPageAction>>,
    preview_button: ViewHandle<ActionButton>,
}

impl PaneFleetNotificationsSettingsPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let path = PaneFleetNotificationPreferences::path();
        let preferences = PaneFleetNotificationPreferences::load_or_default(&path);
        if !path.exists()
            && let Err(error) = preferences.write_atomic(&path)
        {
            log::warn!("Failed to write PaneFleet notification preferences: {error}");
        }

        let selected_sound = preferences.agent_completion_sound;
        let sound_dropdown = ctx.add_typed_action_view(move |ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(SOUND_DROPDOWN_WIDTH);
            dropdown.set_menu_width(SOUND_DROPDOWN_WIDTH, ctx);
            dropdown.set_items(
                PaneFleetNotificationSound::ALL
                    .into_iter()
                    .map(|sound| {
                        DropdownItem::new(
                            sound.display_name(),
                            PaneFleetNotificationsSettingsPageAction::SetAgentCompletionSound(
                                sound,
                            ),
                        )
                    })
                    .collect(),
                ctx,
            );
            dropdown.set_selected_by_action(
                PaneFleetNotificationsSettingsPageAction::SetAgentCompletionSound(selected_sound),
                ctx,
            );
            dropdown
        });
        let preview_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Preview", SecondaryTheme)
                .with_icon(Icon::Play)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(PaneFleetNotificationsSettingsPageAction::Preview);
                })
        });

        Self {
            page: PageType::new_monolith(PaneFleetNotificationsSettingsWidget, None, true),
            preferences,
            enabled_switch: SwitchStateHandle::default(),
            sound_dropdown,
            preview_button,
        }
    }

    fn persist(&self, ctx: &mut ViewContext<Self>) {
        if let Err(error) = self
            .preferences
            .write_atomic(&PaneFleetNotificationPreferences::path())
        {
            log::warn!("Failed to write PaneFleet notification preferences: {error}");
        }
        ctx.notify();
    }

    fn render_enabled_row(&self, appearance: &Appearance) -> Box<dyn Element> {
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
                                "Agent completion sound",
                                appearance.ui_font_family(),
                                appearance.ui_font_size(),
                            )
                            .with_style(Properties::default().weight(Weight::Medium))
                            .with_color(theme.active_ui_text_color().into())
                            .finish(),
                        )
                        .with_child(
                            Text::new_inline(
                                "Play a quiet sound when an agent finishes its current turn.",
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
                        .switch(self.enabled_switch.clone())
                        .check(self.preferences.agent_completion_sound_enabled)
                        .build()
                        .on_click(|ctx, _, _| {
                            ctx.dispatch_typed_action(
                                PaneFleetNotificationsSettingsPageAction::ToggleAgentCompletionSound,
                            );
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

impl Entity for PaneFleetNotificationsSettingsPageView {
    type Event = ();
}

impl View for PaneFleetNotificationsSettingsPageView {
    fn ui_name() -> &'static str {
        "PaneFleetNotificationsSettingsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl TypedActionView for PaneFleetNotificationsSettingsPageView {
    type Action = PaneFleetNotificationsSettingsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            PaneFleetNotificationsSettingsPageAction::ToggleAgentCompletionSound => {
                self.preferences.agent_completion_sound_enabled =
                    !self.preferences.agent_completion_sound_enabled;
                self.persist(ctx);
            }
            PaneFleetNotificationsSettingsPageAction::SetAgentCompletionSound(sound) => {
                self.preferences.agent_completion_sound = *sound;
                self.persist(ctx);
            }
            PaneFleetNotificationsSettingsPageAction::Preview => {
                self.preferences.agent_completion_sound.play();
            }
        }
    }
}

impl SettingsPageMeta for PaneFleetNotificationsSettingsPageView {
    fn section() -> SettingsSection {
        SettingsSection::PaneFleetNotifications
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

impl From<ViewHandle<PaneFleetNotificationsSettingsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<PaneFleetNotificationsSettingsPageView>) -> Self {
        SettingsPageViewHandle::PaneFleetNotifications(view_handle)
    }
}

struct PaneFleetNotificationsSettingsWidget;

impl SettingsWidget for PaneFleetNotificationsSettingsWidget {
    type View = PaneFleetNotificationsSettingsPageView;

    fn search_terms(&self) -> &str {
        "notifications agent completion sound glass pop tink"
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
                Text::new_inline("Notifications", appearance.ui_font_family(), 23.)
                    .with_style(Properties::default().weight(Weight::Bold))
                    .with_color(theme.active_ui_text_color().into())
                    .finish(),
            )
            .with_child(
                Container::new(
                    Text::new_inline(
                        "Choose a lightweight cue for completed CLI-agent turns.",
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
            .with_child(view.render_enabled_row(appearance))
            .with_child(
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
                                        "Sound",
                                        appearance.ui_font_family(),
                                        appearance.ui_font_size(),
                                    )
                                    .with_style(Properties::default().weight(Weight::Medium))
                                    .with_color(theme.active_ui_text_color().into())
                                    .finish(),
                                )
                                .with_child(
                                    Text::new_inline(
                                        "Three subtle sounds provided by macOS.",
                                        appearance.ui_font_family(),
                                        appearance.ui_font_size() - 1.,
                                    )
                                    .with_color(theme.sub_text_color(theme.background()).into())
                                    .finish(),
                                )
                                .finish(),
                        )
                        .with_child(
                            Flex::row()
                                .with_main_axis_size(MainAxisSize::Min)
                                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                .with_child(ChildView::new(&view.sound_dropdown).finish())
                                .with_spacing(8.)
                                .with_child(ChildView::new(&view.preview_button).finish())
                                .finish(),
                        )
                        .finish(),
                )
                .with_padding_top(12.)
                .with_padding_bottom(12.)
                .finish(),
            )
            .finish()
    }
}
