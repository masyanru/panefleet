use warpui::assets::asset_cache::AssetSource;
use warpui::elements::{
    Align, CacheOption, ChildView, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex,
    Image, MainAxisAlignment, MouseStateHandle, ParentElement, Wrap,
};
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, Entity, SingletonEntity, View, ViewContext, ViewHandle};

use super::SettingsSection;
use super::settings_page::{
    MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle,
    SettingsWidget,
};
use crate::appearance::Appearance;
use crate::channel::ChannelState;
use crate::features::FeatureFlag;
use crate::panefleet_update::{
    PaneFleetUpdateChecker, PaneFleetUpdateEvent, PaneFleetUpdateStatus,
};
use crate::themes::theme::ColorScheme;
use crate::ui_components::icons::Icon;
use crate::view_components::action_button::{ActionButton, SecondaryTheme};
use crate::workspace::WorkspaceAction;

pub struct AboutPageView {
    page: PageType<Self>,
    check_updates_button: ViewHandle<ActionButton>,
    open_releases_button: ViewHandle<ActionButton>,
}

impl AboutPageView {
    pub fn new(ctx: &mut ViewContext<AboutPageView>) -> Self {
        let check_updates_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Check for updates", SecondaryTheme)
                .with_icon(Icon::Refresh)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(WorkspaceAction::CheckPaneFleetForUpdates);
                })
        });
        let open_releases_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Open releases", SecondaryTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(WorkspaceAction::OpenLink(
                    "https://github.com/masyanru/panefleet/releases".to_owned(),
                ));
            })
        });
        ctx.subscribe_to_model(
            &PaneFleetUpdateChecker::handle(ctx),
            |_view, _model, event, ctx| {
                if matches!(event, PaneFleetUpdateEvent::Changed) {
                    ctx.notify();
                }
            },
        );

        AboutPageView {
            page: PageType::new_monolith(AboutPageWidget::default(), None, false),
            check_updates_button,
            open_releases_button,
        }
    }
}

impl Entity for AboutPageView {
    type Event = SettingsPageEvent;
}

impl View for AboutPageView {
    fn ui_name() -> &'static str {
        "AboutPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Default)]
struct AboutPageWidget {
    copy_version_button_mouse_state: MouseStateHandle,
}

impl SettingsWidget for AboutPageWidget {
    type View = AboutPageView;

    fn search_terms(&self) -> &str {
        "about panefleet version update"
    }

    fn render(
        &self,
        view: &AboutPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder();
        let is_panefleet = FeatureFlag::PaneFleetWorkbench.is_enabled();

        let image_path = if is_panefleet {
            "bundled/png/panefleet.png"
        } else if theme.inferred_color_scheme() == ColorScheme::LightOnDark {
            "bundled/svg/warp-logo-with-light-title.svg"
        } else {
            "bundled/svg/warp-logo-with-dark-title.svg"
        };

        let version = if is_panefleet {
            PaneFleetUpdateChecker::current_version_label()
        } else {
            ChannelState::app_version().unwrap_or("v#.##.###")
        };

        let version_text = ui_builder
            .span(version.to_string())
            .with_soft_wrap()
            .build()
            .with_margin_top(16.)
            .finish();

        let copy_version_icon = appearance
            .ui_builder()
            .copy_button(16., self.copy_version_button_mouse_state.clone())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::CopyVersion(version));
            })
            .finish();

        let version_row = Wrap::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_children([
                version_text,
                Container::new(copy_version_icon)
                    .with_margin_top(16.)
                    .with_padding_left(6.)
                    .finish(),
            ]);

        let mut content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                ConstrainedBox::new(
                    Image::new(
                        AssetSource::Bundled { path: image_path },
                        CacheOption::BySize,
                    )
                    .finish(),
                )
                .with_max_height(100.)
                .with_max_width(if is_panefleet { 100. } else { 350. })
                .finish(),
            )
            .with_child(version_row.finish());

        if is_panefleet {
            let status = match PaneFleetUpdateChecker::as_ref(app).status() {
                PaneFleetUpdateStatus::Idle => "Updates have not been checked yet.".to_owned(),
                PaneFleetUpdateStatus::Checking => "Checking GitHub Releases…".to_owned(),
                PaneFleetUpdateStatus::UpToDate => "PaneFleet is up to date.".to_owned(),
                PaneFleetUpdateStatus::Available(release) => {
                    format!("Update available: {}", release.tag_name)
                }
                PaneFleetUpdateStatus::Error(error) => {
                    format!("Could not check for updates: {error}")
                }
            };
            content.add_child(
                ui_builder
                    .span(status)
                    .with_soft_wrap()
                    .build()
                    .with_margin_top(16.)
                    .finish(),
            );
            content.add_child(
                Container::new(
                    Flex::row()
                        .with_main_axis_alignment(MainAxisAlignment::Center)
                        .with_child(ChildView::new(&view.check_updates_button).finish())
                        .with_child(
                            Container::new(ChildView::new(&view.open_releases_button).finish())
                                .with_margin_left(8.)
                                .finish(),
                        )
                        .finish(),
                )
                .with_margin_top(16.)
                .finish(),
            );
            content.add_child(
                ui_builder
                    .span("PaneFleet is open source under the AGPL-3.0 license.")
                    .build()
                    .with_margin_top(16.)
                    .finish(),
            );
        } else {
            content.add_child(
                ui_builder
                    .span("Copyright 2026 Warp")
                    .build()
                    .with_margin_top(16.)
                    .finish(),
            );
        }

        Align::new(content.finish()).finish()
    }
}

impl SettingsPageMeta for AboutPageView {
    fn section() -> SettingsSection {
        SettingsSection::About
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
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

impl From<ViewHandle<AboutPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AboutPageView>) -> Self {
        SettingsPageViewHandle::About(view_handle)
    }
}
