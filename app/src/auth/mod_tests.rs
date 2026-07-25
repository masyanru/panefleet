use super::web_logout_url_for_server_root;

#[test]
fn web_logout_url_uses_configured_server_root() {
    assert_eq!(
        web_logout_url_for_server_root("https://staging.warp.dev/"),
        "https://staging.warp.dev/logout"
    );
    assert_eq!(
        web_logout_url_for_server_root("http://localhost:8080"),
        "http://localhost:8080/logout"
    );
}
