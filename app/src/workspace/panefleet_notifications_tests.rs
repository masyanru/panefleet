use super::{
    PANEFLEET_NOTIFICATION_PREFERENCES_VERSION, PaneFleetNotificationPreferences,
    PaneFleetNotificationSound,
};

#[test]
fn defaults_to_a_quiet_enabled_completion_sound() {
    let preferences = PaneFleetNotificationPreferences::default();

    assert!(preferences.agent_completion_sound_enabled);
    assert_eq!(
        preferences.agent_completion_sound,
        PaneFleetNotificationSound::Glass
    );
}

#[test]
fn older_files_receive_new_notification_defaults() {
    let preferences =
        PaneFleetNotificationPreferences::decode(br#"{"version":1}"#).expect("valid preferences");

    assert!(preferences.agent_completion_sound_enabled);
    assert_eq!(
        preferences.agent_completion_sound,
        PaneFleetNotificationSound::Glass
    );
}

#[test]
fn future_preference_versions_are_rejected_by_loader() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("notifications.json");
    std::fs::write(
        &path,
        format!(
            r#"{{"version":{},"agent_completion_sound_enabled":false,"agent_completion_sound":"pop"}}"#,
            PANEFLEET_NOTIFICATION_PREFERENCES_VERSION + 1
        ),
    )
    .expect("write preferences");

    assert_eq!(
        PaneFleetNotificationPreferences::load_or_default(&path),
        PaneFleetNotificationPreferences::default()
    );
}
