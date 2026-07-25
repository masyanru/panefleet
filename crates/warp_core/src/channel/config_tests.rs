use url::Url;

use super::{OzConfig, WarpServerConfig};

#[test]
fn local_only_server_config_never_targets_an_external_host() {
    let config = WarpServerConfig::local_only();

    for endpoint in [&config.server_root_url, &config.rtc_server_url] {
        let url = Url::parse(endpoint).expect("local-only endpoint should be a valid URL");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
    }
    assert_eq!(config.session_sharing_server_url, None);
    assert_eq!(config.firebase_auth_api_key, "");
}

#[test]
fn local_only_oz_config_never_targets_an_external_host() {
    let config = OzConfig::local_only();
    let url =
        Url::parse(&config.oz_root_url).expect("local-only Oz endpoint should be a valid URL");

    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(config.workload_audience_url, None);
}
