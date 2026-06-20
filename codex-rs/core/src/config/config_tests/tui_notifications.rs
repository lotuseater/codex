use super::*;
use pretty_assertions::assert_eq;

struct TuiTomlTest {
    #[serde(default, flatten)]
    notifications: TuiNotificationSettings,
}

#[derive(Deserialize, Debug, PartialEq)]
struct RootTomlTest {
    tui: TuiTomlTest,
}

#[test]
fn test_tui_notifications_true() {
    let toml = r#"
            [tui]
            notifications = true
        "#;
    let parsed: RootTomlTest = toml::from_str(toml).expect("deserialize notifications=true");
    assert_matches!(
        parsed.tui.notifications.notifications,
        Notifications::Enabled(true)
    );
}

#[test]
fn test_tui_notifications_custom_array() {
    let toml = r#"
            [tui]
            notifications = ["foo"]
        "#;
    let parsed: RootTomlTest = toml::from_str(toml).expect("deserialize notifications=[\"foo\"]");
    assert_matches!(
        parsed.tui.notifications.notifications,
        Notifications::Custom(ref v) if v == &vec!["foo".to_string()]
    );
}

#[test]
fn test_tui_notification_method() {
    let toml = r#"
            [tui]
            notification_method = "bel"
        "#;
    let parsed: RootTomlTest =
        toml::from_str(toml).expect("deserialize notification_method=\"bel\"");
    assert_eq!(parsed.tui.notifications.method, NotificationMethod::Bel);
}

#[test]
fn test_tui_notification_condition_defaults_to_unfocused() {
    let toml = r#"
            [tui]
        "#;
    let parsed: RootTomlTest =
        toml::from_str(toml).expect("deserialize default notification condition");
    assert_eq!(
        parsed.tui.notifications.condition,
        NotificationCondition::Unfocused
    );
}

#[test]
fn test_tui_notification_condition_always() {
    let toml = r#"
            [tui]
            notification_condition = "always"
        "#;
    let parsed: RootTomlTest =
        toml::from_str(toml).expect("deserialize notification_condition=\"always\"");
    assert_eq!(
        parsed.tui.notifications.condition,
        NotificationCondition::Always
    );
}

#[test]
fn test_tui_notification_condition_rejects_unknown_value() {
    let toml = r#"
            [tui]
            notification_condition = "background"
        "#;
    let err = toml::from_str::<RootTomlTest>(toml).expect_err("reject unknown condition");
    let err = err.to_string();
    assert!(
        err.contains("unknown variant `background`")
            && err.contains("unfocused")
            && err.contains("always"),
        "unexpected error: {err}"
    );
}
