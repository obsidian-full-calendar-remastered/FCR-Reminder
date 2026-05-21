use reminder_core::Reminder;
use std::error::Error;

/// Dispatches a rich interactive Windows Toast notification using Windows Runtime APIs.
pub fn trigger_notification(reminder: &Reminder) -> Result<(), Box<dyn Error>> {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

    let app_id = HSTRING::from("FCRReminder");

    let title_esc = xml_escape(&reminder.title);
    let body_esc = xml_escape(&reminder.body);

    let id_enc = url_encode(&reminder.id);
    let title_enc = url_encode(&reminder.title);
    let body_enc = url_encode(&reminder.body);
    let action_url_enc = url_encode(&reminder.action_url);

    let snooze_args = format!(
        "fcr-reminder://snooze?id={}&title={}&body={}&action_url={}",
        id_enc, title_enc, body_enc, action_url_enc
    );
    let snooze_args_esc = xml_escape(&snooze_args);

    let open_note_action = if !reminder.action_url.is_empty() {
        let action_url_esc = xml_escape(&reminder.action_url);
        format!(
            "<action content=\"Open Note\" activationType=\"protocol\" arguments=\"{}\"/>",
            action_url_esc
        )
    } else {
        String::new()
    };

    let xml_content = format!(
        r#"<toast duration="long">
    <visual>
        <binding template="ToastGeneric">
            <text>{}</text>
            <text>{}</text>
        </binding>
    </visual>
    <audio src="ms-winsoundevent:Notification.Reminder"/>
    <actions>
        <input id="snoozeTime" type="selection" defaultInput="5">
            <selection id="5" content="5 minutes"/>
            <selection id="10" content="10 minutes"/>
            <selection id="15" content="15 minutes"/>
            <selection id="30" content="30 minutes"/>
            <selection id="60" content="1 hour"/>
        </input>
        <action
            content="Snooze"
            activationType="protocol"
            arguments="{}"
            hint-inputId="snoozeTime"/>
        {}
    </actions>
</toast>"#,
        title_esc, body_esc, snooze_args_esc, open_note_action
    );

    let xml_doc = XmlDocument::new()?;
    xml_doc.LoadXml(&HSTRING::from(&xml_content))?;

    let toast = ToastNotification::CreateToastNotification(&xml_doc)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&app_id)?;
    notifier.Show(&toast)?;

    Ok(())
}

fn xml_escape(input: &str) -> String {
    let mut escaped = String::new();
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}