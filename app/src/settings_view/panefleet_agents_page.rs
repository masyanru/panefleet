use warp_core::features::FeatureFlag;
use warpui::elements::{
    Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Empty, Flex,
    MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement, Radius, Shrinkable, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::ui_components::button::ButtonVariant;
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::settings_page::{
    MatchData, PageType, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
};
use super::{SettingsSection, editor_text_colors};
use crate::appearance::Appearance;
use crate::editor::{EditorView, Event as EditorEvent, SingleLineEditorOptions, TextOptions};
use crate::terminal::CLIAgent;
use crate::ui_components::icons::Icon;
use crate::view_components::Dropdown;
use crate::view_components::action_button::{
    ActionButton, DangerSecondaryTheme, PrimaryTheme, SecondaryTheme,
};
use crate::view_components::dropdown::DropdownItem;
use crate::workspace::panefleet_agents::{
    PaneFleetAgentDefinition, PaneFleetAgentDefinitions, PaneFleetPromptTransport,
};

const AGENT_LIST_WIDTH: f32 = 210.;
const DROPDOWN_WIDTH: f32 = 220.;

#[derive(Debug, Clone)]
pub enum PaneFleetAgentsSettingsPageEvent {
    DefinitionsChanged,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneFleetAgentsSettingsPageAction {
    SelectAgent(String),
    AddAgent,
    Save,
    RestoreDefault,
    Delete,
    ToggleLauncher,
    SetAgentKind(CLIAgent),
    SetPromptTransport(PaneFleetPromptTransport),
}

pub struct PaneFleetAgentsSettingsPageView {
    page: PageType<Self>,
    definitions: PaneFleetAgentDefinitions,
    selected_id: Option<String>,
    definition_row_mouse_states: Vec<MouseStateHandle>,
    launcher_switch_state: SwitchStateHandle,
    label_editor: ViewHandle<EditorView>,
    executable_editor: ViewHandle<EditorView>,
    args_editor: ViewHandle<EditorView>,
    prompt_only_args_editor: ViewHandle<EditorView>,
    launcher_order_editor: ViewHandle<EditorView>,
    agent_kind_dropdown: ViewHandle<Dropdown<PaneFleetAgentsSettingsPageAction>>,
    prompt_transport_dropdown: ViewHandle<Dropdown<PaneFleetAgentsSettingsPageAction>>,
    add_button: ViewHandle<ActionButton>,
    save_button: ViewHandle<ActionButton>,
    restore_button: ViewHandle<ActionButton>,
    delete_button: ViewHandle<ActionButton>,
    validation_error: Option<String>,
}

impl PaneFleetAgentsSettingsPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let path = Self::definitions_path();
        let definitions = PaneFleetAgentDefinitions::load_or_default(&path);
        if !path.exists()
            && let Err(error) = definitions.write_atomic(&path)
        {
            log::warn!("Failed to write PaneFleet agent definitions: {error}");
        }
        let selected_id = definitions
            .definitions
            .first()
            .map(|definition| definition.id.clone());

        let label_editor = Self::create_editor("Agent label", ctx);
        let executable_editor = Self::create_editor("Executable, e.g. claude", ctx);
        let args_editor = Self::create_editor("Arguments, separated like a shell command", ctx);
        let prompt_only_args_editor =
            Self::create_editor("Arguments used only with an initial prompt", ctx);
        let launcher_order_editor = Self::create_editor("10", ctx);

        let agent_kind_dropdown = ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(DROPDOWN_WIDTH);
            dropdown.set_menu_width(DROPDOWN_WIDTH, ctx);
            dropdown.set_items(
                [CLIAgent::Codex, CLIAgent::Claude, CLIAgent::OpenCode]
                    .into_iter()
                    .map(|agent| {
                        DropdownItem::new(
                            agent.display_name(),
                            PaneFleetAgentsSettingsPageAction::SetAgentKind(agent),
                        )
                    })
                    .collect(),
                ctx,
            );
            dropdown
        });

        let prompt_transport_dropdown = ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(DROPDOWN_WIDTH);
            dropdown.set_menu_width(DROPDOWN_WIDTH, ctx);
            dropdown.set_items(
                vec![
                    DropdownItem::new(
                        "argv",
                        PaneFleetAgentsSettingsPageAction::SetPromptTransport(
                            PaneFleetPromptTransport::Argv,
                        ),
                    ),
                    DropdownItem::new(
                        "stdin",
                        PaneFleetAgentsSettingsPageAction::SetPromptTransport(
                            PaneFleetPromptTransport::Stdin,
                        ),
                    ),
                ],
                ctx,
            );
            dropdown
        });

        let add_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Add agent", SecondaryTheme)
                .with_icon(Icon::Plus)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(PaneFleetAgentsSettingsPageAction::AddAgent);
                })
        });
        let save_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Save", PrimaryTheme)
                .with_icon(Icon::Check)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(PaneFleetAgentsSettingsPageAction::Save);
                })
        });
        let restore_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Restore default", SecondaryTheme)
                .with_icon(Icon::Refresh)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(PaneFleetAgentsSettingsPageAction::RestoreDefault);
                })
        });
        let delete_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Delete agent", DangerSecondaryTheme)
                .with_icon(Icon::Trash)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(PaneFleetAgentsSettingsPageAction::Delete);
                })
        });

        let mut view = Self {
            page: PageType::new_monolith(PaneFleetAgentsSettingsWidget, None, true),
            definition_row_mouse_states: definitions
                .definitions
                .iter()
                .map(|_| MouseStateHandle::default())
                .collect(),
            definitions,
            selected_id,
            launcher_switch_state: SwitchStateHandle::default(),
            label_editor,
            executable_editor,
            args_editor,
            prompt_only_args_editor,
            launcher_order_editor,
            agent_kind_dropdown,
            prompt_transport_dropdown,
            add_button,
            save_button,
            restore_button,
            delete_button,
            validation_error: None,
        };
        view.sync_form(ctx);
        view
    }

    fn definitions_path() -> std::path::PathBuf {
        warp_core::paths::state_dir().join("panefleet-agent-definitions.json")
    }

    fn create_editor(
        placeholder: &'static str,
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<EditorView> {
        let appearance = Appearance::as_ref(ctx);
        let options = SingleLineEditorOptions {
            text: TextOptions {
                text_colors_override: Some(editor_text_colors(appearance)),
                ..TextOptions::ui_font_size(appearance)
            },
            ..Default::default()
        };
        let editor = ctx.add_typed_action_view(move |ctx| {
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(placeholder, ctx);
            editor
        });
        ctx.subscribe_to_view(&editor, |me, _, event, ctx| {
            if matches!(event, EditorEvent::Edited(_)) {
                me.validation_error = None;
                ctx.notify();
            }
        });
        editor
    }

    fn selected_definition(&self) -> Option<&PaneFleetAgentDefinition> {
        let selected_id = self.selected_id.as_deref()?;
        self.definitions
            .definitions
            .iter()
            .find(|definition| definition.id == selected_id)
    }

    fn selected_definition_mut(&mut self) -> Option<&mut PaneFleetAgentDefinition> {
        let selected_id = self.selected_id.as_deref()?;
        self.definitions
            .definitions
            .iter_mut()
            .find(|definition| definition.id == selected_id)
    }

    fn sync_form(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(definition) = self.selected_definition().cloned() else {
            return;
        };
        let fields = [
            (&self.label_editor, definition.label),
            (&self.executable_editor, definition.executable),
            (
                &self.args_editor,
                shell_words::join(definition.args.iter().map(String::as_str)),
            ),
            (
                &self.prompt_only_args_editor,
                shell_words::join(definition.prompt_only_args.iter().map(String::as_str)),
            ),
            (
                &self.launcher_order_editor,
                definition.launcher_order.to_string(),
            ),
        ];
        for (editor, value) in fields {
            editor.update(ctx, |editor, ctx| {
                editor.system_reset_buffer_text(&value, ctx);
            });
        }
        self.agent_kind_dropdown.update(ctx, |dropdown, ctx| {
            dropdown.set_selected_by_action(
                PaneFleetAgentsSettingsPageAction::SetAgentKind(definition.agent),
                ctx,
            );
        });
        self.prompt_transport_dropdown.update(ctx, |dropdown, ctx| {
            dropdown.set_selected_by_action(
                PaneFleetAgentsSettingsPageAction::SetPromptTransport(definition.prompt_transport),
                ctx,
            );
        });
        self.validation_error = None;
        ctx.notify();
    }

    fn editor_text(editor: &ViewHandle<EditorView>, ctx: &AppContext) -> String {
        editor.as_ref(ctx).buffer_text(ctx).trim().to_string()
    }

    fn save(&mut self, ctx: &mut ViewContext<Self>) {
        let label = Self::editor_text(&self.label_editor, ctx);
        let executable = Self::editor_text(&self.executable_editor, ctx);
        if label.is_empty() || executable.is_empty() {
            self.validation_error = Some("Label and executable cannot be empty.".to_string());
            ctx.notify();
            return;
        }

        let args = match shell_words::split(&Self::editor_text(&self.args_editor, ctx)) {
            Ok(args) => args,
            Err(error) => {
                self.validation_error = Some(format!("Invalid launch arguments: {error}"));
                ctx.notify();
                return;
            }
        };
        let prompt_only_args =
            match shell_words::split(&Self::editor_text(&self.prompt_only_args_editor, ctx)) {
                Ok(args) => args,
                Err(error) => {
                    self.validation_error = Some(format!("Invalid prompt-only arguments: {error}"));
                    ctx.notify();
                    return;
                }
            };
        let launcher_order = match Self::editor_text(&self.launcher_order_editor, ctx).parse() {
            Ok(order) => order,
            Err(_) => {
                self.validation_error =
                    Some("Launcher order must be a non-negative integer.".to_string());
                ctx.notify();
                return;
            }
        };

        if let Some(definition) = self.selected_definition_mut() {
            definition.label = label;
            definition.executable = executable;
            definition.args = args;
            definition.prompt_only_args = prompt_only_args;
            definition.launcher_order = launcher_order;
        }
        self.persist(ctx);
    }

    fn persist(&mut self, ctx: &mut ViewContext<Self>) {
        match self.definitions.write_atomic(&Self::definitions_path()) {
            Ok(()) => {
                self.validation_error = None;
                ctx.emit(PaneFleetAgentsSettingsPageEvent::DefinitionsChanged);
            }
            Err(error) => {
                self.validation_error = Some(format!("Could not save agent settings: {error}"));
            }
        }
        ctx.notify();
    }

    fn add_agent(&mut self, ctx: &mut ViewContext<Self>) {
        let launcher_order = self
            .definitions
            .definitions
            .iter()
            .map(|definition| definition.launcher_order)
            .max()
            .unwrap_or_default()
            .saturating_add(10);
        let id = format!("custom.{}", uuid::Uuid::new_v4());
        self.definitions.definitions.push(PaneFleetAgentDefinition {
            id: id.clone(),
            label: "Custom agent".to_string(),
            agent: CLIAgent::Claude,
            executable: "claude".to_string(),
            args: Vec::new(),
            prompt_only_args: Vec::new(),
            prompt_transport: PaneFleetPromptTransport::Argv,
            enabled_in_launcher: true,
            launcher_order,
            bundled: false,
        });
        self.definition_row_mouse_states
            .push(MouseStateHandle::default());
        self.selected_id = Some(id);
        self.persist(ctx);
        self.sync_form(ctx);
        ctx.focus(&self.label_editor);
    }

    fn restore_default(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(selected_id) = self.selected_id.clone() else {
            return;
        };
        let Some(default) = PaneFleetAgentDefinitions::bundled_default(&selected_id) else {
            return;
        };
        if let Some(definition) = self.selected_definition_mut() {
            *definition = default;
        }
        self.persist(ctx);
        self.sync_form(ctx);
    }

    fn delete(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(selected_id) = self.selected_id.clone() else {
            return;
        };
        if self
            .selected_definition()
            .is_none_or(|definition| definition.bundled)
        {
            return;
        }
        if let Some(index) = self
            .definitions
            .definitions
            .iter()
            .position(|definition| definition.id == selected_id)
        {
            self.definitions.definitions.remove(index);
            self.definition_row_mouse_states.remove(index);
        }
        self.selected_id = self
            .definitions
            .definitions
            .first()
            .map(|definition| definition.id.clone());
        self.persist(ctx);
        self.sync_form(ctx);
    }

    fn render_agent_list(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let mut list = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(4.)
            .with_child(
                Container::new(ChildView::new(&self.add_button).finish())
                    .with_padding_bottom(8.)
                    .finish(),
            );
        for (index, definition) in self.definitions.definitions.iter().enumerate() {
            let selected = self.selected_id.as_deref() == Some(definition.id.as_str());
            let id = definition.id.clone();
            let icon = definition.agent.icon().unwrap_or(Icon::Terminal);
            let label = Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(8.)
                .with_child(
                    ConstrainedBox::new(icon.to_warpui_icon(theme.active_ui_text_color()).finish())
                        .with_width(16.)
                        .with_height(16.)
                        .finish(),
                )
                .with_child(
                    Shrinkable::new(
                        1.,
                        Text::new_inline(
                            definition.label.clone(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(theme.active_ui_text_color().into())
                        .finish(),
                    )
                    .finish(),
                )
                .finish();
            list.add_child(
                appearance
                    .ui_builder()
                    .button(
                        if selected {
                            ButtonVariant::Accent
                        } else {
                            ButtonVariant::Text
                        },
                        self.definition_row_mouse_states[index].clone(),
                    )
                    .with_custom_label(label)
                    .with_style(UiComponentStyles {
                        padding: Some(Coords::uniform(8.)),
                        ..Default::default()
                    })
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(PaneFleetAgentsSettingsPageAction::SelectAgent(
                            id.clone(),
                        ));
                    })
                    .finish(),
            );
        }

        ConstrainedBox::new(
            Container::new(list.finish())
                .with_padding_right(12.)
                .with_border(Border::right(1.).with_border_fill(theme.outline()))
                .finish(),
        )
        .with_width(AGENT_LIST_WIDTH)
        .finish()
    }

    fn render_field(
        &self,
        appearance: &Appearance,
        label: &'static str,
        description: &'static str,
        editor: &ViewHandle<EditorView>,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        Flex::column()
            .with_child(
                Text::new_inline(
                    label,
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_style(Properties::default().weight(Weight::Medium))
                .with_color(theme.active_ui_text_color().into())
                .finish(),
            )
            .with_child(
                Container::new(
                    Text::new_inline(
                        description,
                        appearance.ui_font_family(),
                        appearance.ui_font_size() - 1.,
                    )
                    .with_color(theme.sub_text_color(theme.background()).into())
                    .finish(),
                )
                .with_margin_top(3.)
                .with_margin_bottom(6.)
                .finish(),
            )
            .with_child(
                appearance
                    .ui_builder()
                    .text_input(editor.clone())
                    .with_style(UiComponentStyles {
                        background: Some(theme.surface_2().into()),
                        font_color: Some(theme.main_text_color(theme.surface_2()).into_solid()),
                        padding: Some(Coords::uniform(8.)),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
            )
            .with_spacing(0.)
            .finish()
    }

    fn render_editor(&self, appearance: &Appearance) -> Box<dyn Element> {
        let Some(definition) = self.selected_definition() else {
            return Container::new(Empty::new().finish()).finish();
        };
        let theme = appearance.theme();
        let mut form = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(18.)
            .with_child(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(10.)
                    .with_child(
                        ConstrainedBox::new(
                            definition
                                .agent
                                .icon()
                                .unwrap_or(Icon::Terminal)
                                .to_warpui_icon(theme.active_ui_text_color())
                                .finish(),
                        )
                        .with_width(24.)
                        .with_height(24.)
                        .finish(),
                    )
                    .with_child(
                        Text::new_inline(
                            definition.label.clone(),
                            appearance.ui_font_family(),
                            18.,
                        )
                        .with_style(Properties::default().weight(Weight::Bold))
                        .with_color(theme.active_ui_text_color().into())
                        .finish(),
                    )
                    .finish(),
            )
            .with_child(self.render_field(
                appearance,
                "Label",
                "Name shown in the launcher and tab title.",
                &self.label_editor,
            ))
            .with_child(
                Flex::column()
                    .with_spacing(6.)
                    .with_child(
                        Text::new_inline(
                            "Resume adapter",
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_style(Properties::default().weight(Weight::Medium))
                        .with_color(theme.active_ui_text_color().into())
                        .finish(),
                    )
                    .with_child(
                        Text::new_inline(
                            "Selects the CLI-specific launch, identity detection, and resume behavior.",
                            appearance.ui_font_family(),
                            appearance.ui_font_size() - 1.,
                        )
                        .with_color(theme.sub_text_color(theme.background()).into())
                        .finish(),
                    )
                    .with_child(ChildView::new(&self.agent_kind_dropdown).finish())
                    .finish(),
            )
            .with_child(self.render_field(
                appearance,
                "Executable",
                "Executable used to start a new session.",
                &self.executable_editor,
            ))
            .with_child(self.render_field(
                appearance,
                "Launch arguments",
                "Shell-style argv appended to every launch. Dangerous permission flags remain visible here.",
                &self.args_editor,
            ))
            .with_child(self.render_field(
                appearance,
                "Prompt-only arguments",
                "Arguments added only when PaneFleet launches the agent with an initial prompt.",
                &self.prompt_only_args_editor,
            ))
            .with_child(
                Flex::column()
                    .with_spacing(6.)
                    .with_child(
                        Text::new_inline(
                            "Prompt transport",
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_style(Properties::default().weight(Weight::Medium))
                        .with_color(theme.active_ui_text_color().into())
                        .finish(),
                    )
                    .with_child(
                        Text::new_inline(
                            "Deliver the initial prompt as argv or through stdin.",
                            appearance.ui_font_family(),
                            appearance.ui_font_size() - 1.,
                        )
                        .with_color(theme.sub_text_color(theme.background()).into())
                        .finish(),
                    )
                    .with_child(ChildView::new(&self.prompt_transport_dropdown).finish())
                    .finish(),
            )
            .with_child(self.render_field(
                appearance,
                "Launcher order",
                "Lower values appear first in the launcher bar.",
                &self.launcher_order_editor,
            ))
            .with_child(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(
                        Flex::column()
                            .with_child(
                                Text::new_inline(
                                    "Enabled in launcher",
                                    appearance.ui_font_family(),
                                    appearance.ui_font_size(),
                                )
                                .with_style(Properties::default().weight(Weight::Medium))
                                .with_color(theme.active_ui_text_color().into())
                                .finish(),
                            )
                            .with_child(
                                Text::new_inline(
                                    "Show this definition below the horizontal tabs.",
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
                            .switch(self.launcher_switch_state.clone())
                            .check(definition.enabled_in_launcher)
                            .build()
                            .on_click(|ctx, _, _| {
                                ctx.dispatch_typed_action(
                                    PaneFleetAgentsSettingsPageAction::ToggleLauncher,
                                );
                            })
                            .finish(),
                    )
                    .finish(),
            );

        if let Some(error) = &self.validation_error {
            form.add_child(
                Container::new(
                    Text::new_inline(
                        error.clone(),
                        appearance.ui_font_family(),
                        appearance.ui_font_size(),
                    )
                    .with_color(theme.ui_error_color().into())
                    .finish(),
                )
                .with_uniform_padding(8.)
                .with_border(Border::all(1.).with_border_fill(theme.ui_error_color()))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                .finish(),
            );
        }

        let mut actions = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::End)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.);
        if definition.bundled {
            actions.add_child(ChildView::new(&self.restore_button).finish());
        } else {
            actions.add_child(ChildView::new(&self.delete_button).finish());
        }
        actions.add_child(ChildView::new(&self.save_button).finish());
        form.add_child(actions.finish());

        Container::new(form.finish())
            .with_padding_left(20.)
            .with_padding_bottom(12.)
            .finish()
    }
}

impl Entity for PaneFleetAgentsSettingsPageView {
    type Event = PaneFleetAgentsSettingsPageEvent;
}

impl View for PaneFleetAgentsSettingsPageView {
    fn ui_name() -> &'static str {
        "PaneFleetAgentsSettingsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl TypedActionView for PaneFleetAgentsSettingsPageView {
    type Action = PaneFleetAgentsSettingsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            PaneFleetAgentsSettingsPageAction::SelectAgent(id) => {
                self.selected_id = Some(id.clone());
                self.sync_form(ctx);
            }
            PaneFleetAgentsSettingsPageAction::AddAgent => self.add_agent(ctx),
            PaneFleetAgentsSettingsPageAction::Save => self.save(ctx),
            PaneFleetAgentsSettingsPageAction::RestoreDefault => self.restore_default(ctx),
            PaneFleetAgentsSettingsPageAction::Delete => self.delete(ctx),
            PaneFleetAgentsSettingsPageAction::ToggleLauncher => {
                if let Some(definition) = self.selected_definition_mut() {
                    definition.enabled_in_launcher = !definition.enabled_in_launcher;
                }
                self.persist(ctx);
            }
            PaneFleetAgentsSettingsPageAction::SetAgentKind(agent) => {
                if let Some(definition) = self.selected_definition_mut() {
                    definition.agent = *agent;
                }
                ctx.notify();
            }
            PaneFleetAgentsSettingsPageAction::SetPromptTransport(transport) => {
                if let Some(definition) = self.selected_definition_mut() {
                    definition.prompt_transport = *transport;
                }
                ctx.notify();
            }
        }
    }
}

impl SettingsPageMeta for PaneFleetAgentsSettingsPageView {
    fn section() -> SettingsSection {
        SettingsSection::PaneFleetAgents
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

impl From<ViewHandle<PaneFleetAgentsSettingsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<PaneFleetAgentsSettingsPageView>) -> Self {
        SettingsPageViewHandle::PaneFleetAgents(view_handle)
    }
}

struct PaneFleetAgentsSettingsWidget;

impl SettingsWidget for PaneFleetAgentsSettingsWidget {
    type View = PaneFleetAgentsSettingsPageView;

    fn search_terms(&self) -> &str {
        "agents cli terminal codex claude opencode launch command arguments resume session prompt"
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
                Text::new_inline("CLI agents", appearance.ui_font_family(), 23.)
                    .with_style(Properties::default().weight(Weight::Bold))
                    .with_color(theme.active_ui_text_color().into())
                    .finish(),
            )
            .with_child(
                Container::new(
                    Text::new_inline(
                        "Configure how PaneFleet launches and resumes agent sessions. Changes apply to new tabs immediately.",
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
            .with_child(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(view.render_agent_list(appearance))
                    .with_child(
                        Shrinkable::new(1., view.render_editor(appearance)).finish(),
                    )
                    .finish(),
            )
            .finish()
    }
}
