use std::path::PathBuf;

use uuid::Uuid;
use warp::tui_export::{
    TuiMcpConfigState, TuiMcpServerId, TuiMcpServerSnapshot, TuiMcpServerStatus, TuiMcpSnapshot,
    TuiMcpTransport, register_tui_session_view_test_singletons,
};
use warpui::EntityIdMap;
use warpui_core::elements::tui::{
    TuiBuffer, TuiConstraint, TuiElement, TuiLayoutContext, TuiPaintContext, TuiPaintSurface,
    TuiRect, TuiScreenPosition, TuiSize,
};
use warpui_core::{App, AppContext};

use super::mcp_status_label;
use crate::tui_builder::TuiUiBuilder;

fn server(id: u64, status: TuiMcpServerStatus) -> TuiMcpServerSnapshot {
    TuiMcpServerSnapshot {
        id: TuiMcpServerId(id),
        installation_uuid: Uuid::from_u128(id as u128),
        name: format!("server-{id}"),
        transport: TuiMcpTransport::Stdio,
        status,
        tool_count: 2,
        resource_count: 0,
        has_credentials: false,
        authorization_url: None,
    }
}

#[test]
fn mcp_summary_keeps_missing_config_action_short() {
    let snapshot = TuiMcpSnapshot {
        config_path: PathBuf::from("/tmp/.mcp.json"),
        config_state: TuiMcpConfigState::Missing,
        servers: Vec::new(),
    };

    assert_eq!(
        mcp_status_label(&snapshot),
        ("Not configured · /mcp".to_string(), false)
    );
}

#[test]
fn mcp_summary_reports_mixed_runtime_states() {
    let snapshot = TuiMcpSnapshot {
        config_path: PathBuf::from("/tmp/.mcp.json"),
        config_state: TuiMcpConfigState::Ready,
        servers: vec![
            server(1, TuiMcpServerStatus::Running),
            server(2, TuiMcpServerStatus::Starting),
            server(3, TuiMcpServerStatus::Authenticating),
            server(4, TuiMcpServerStatus::Stopping),
            server(
                5,
                TuiMcpServerStatus::Failed {
                    message: "failed".to_string(),
                },
            ),
            server(6, TuiMcpServerStatus::Offline),
        ],
    };

    assert_eq!(
        mcp_status_label(&snapshot),
        (
            "1 connected · 1 starting · 1 needs auth · 1 stopping · 1 failed · 1 offline · /mcp"
                .to_string(),
            false
        )
    );
}

#[test]
fn mcp_summary_marks_config_errors() {
    let snapshot = TuiMcpSnapshot {
        config_path: PathBuf::from("/tmp/.mcp.json"),
        config_state: TuiMcpConfigState::Invalid {
            message: "invalid JSON".to_string(),
        },
        servers: Vec::new(),
    };

    assert_eq!(
        mcp_status_label(&snapshot),
        ("Config error · run /mcp".to_string(), true)
    );
}

fn render_element_lines(
    mut element: Box<dyn TuiElement>,
    ctx: &AppContext,
    width: u16,
    height: u16,
) -> Vec<String> {
    let mut rendered_views = EntityIdMap::default();
    let mut layout_ctx = TuiLayoutContext {
        rendered_views: &mut rendered_views,
    };
    let size = element.layout(
        TuiConstraint::loose(TuiSize::new(width, height)),
        &mut layout_ctx,
        ctx,
    );
    let area = TuiRect::new(0, 0, size.width, size.height);
    let mut buffer = TuiBuffer::empty(area);
    let mut paint_ctx = TuiPaintContext::new(&mut rendered_views);
    {
        let mut surface = TuiPaintSurface::new(&mut buffer);
        element.render(
            TuiScreenPosition::new(i32::from(area.x), i32::from(area.y)),
            &mut surface,
            &mut paint_ctx,
        );
    }
    buffer.to_lines()
}

#[test]
fn login_line_shows_signed_in_account_email() {
    App::test((), |mut app| async move {
        register_tui_session_view_test_singletons(&mut app);

        let lines = app.read(|ctx| {
            let builder = TuiUiBuilder::from_app(ctx);
            render_element_lines(super::render_login_line(&builder, ctx), ctx, 48, 1)
        });
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Signed in as test_user@warp.dev")),
            "zero-state login line should show the signed-in email:\n{}",
            lines.join("\n")
        );
    });
}
