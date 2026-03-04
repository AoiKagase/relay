use crate::db::Db;
use std::sync::Arc;
use teloxide::{
    dispatching::{Dispatcher, UpdateFilterExt},
    requests::Requester,
    types::{Message, Update},
    utils::command::BotCommands,
    Bot,
};

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are for administering AodeRelay"
)]
enum Command {
    #[command(description = "Display this text.")]
    Start,

    #[command(description = "Display this text.")]
    Help,

    #[command(description = "Block a domain from the relay.")]
    Block { domain: String },

    #[command(description = "Unblock a domain from the relay.")]
    Unblock { domain: String },

    #[command(description = "Allow a domain to connect to the relay (for RESTRICTED_MODE)")]
    Allow { domain: String },

    #[command(description = "Disallow a domain to connect to the relay (for RESTRICTED_MODE)")]
    Disallow { domain: String },

    #[command(description = "List blocked domains")]
    ListBlocks,

    #[command(description = "List allowed domains")]
    ListAllowed,

    #[command(description = "List connected domains")]
    ListConnected,
}

#[derive(Debug)]
struct ParseHostError(Option<Box<dyn std::error::Error + Send + Sync + 'static>>);

impl std::fmt::Display for ParseHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Invalid host provided")
    }
}

impl std::error::Error for ParseHostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.as_deref().map(|e| e as _)
    }
}

fn parse_host(input: &str) -> Result<String, ParseHostError> {
    match (url::Host::parse(input), url::Url::parse(input)) {
        (Ok(host), _) => Ok(host.to_string()),
        (Err(_), Ok(url)) => url
            .host()
            .ok_or(ParseHostError(None))
            .map(|h| h.to_string()),
        (Err(e), Err(_)) => Err(ParseHostError(Some(Box::new(e)))),
    }
}

#[test]
fn parse_host_parses_hosts() {
    let cases = [
        ("https://example.com", "example.com"),
        ("https://masto.asonix.dog", "masto.asonix.dog"),
        ("example.com", "example.com"),
        ("asonix.dog", "asonix.dog"),
        ("http://192.168.1.1", "192.168.1.1"),
        ("http://192.168.1.1:80", "192.168.1.1"),
        ("http://[2001:db8::1]", "[2001:db8::1]"),
        ("http://[2001:db8::1]:443", "[2001:db8::1]"),
        ("192.168.1.1", "192.168.1.1"),
        ("[2001:db8::1]", "[2001:db8::1]"),
    ];

    for (input, expected) in cases {
        let output = parse_host(input).expect("valid host");

        assert_eq!(output, expected, "Failed parsing input {input}");
    }
}

pub(crate) fn start(admin_handle: String, db: Db, token: &str) {
    let bot = Bot::new(token);
    let admin_handle = Arc::new(admin_handle);

    tokio::spawn(async move {
        let command_handler = teloxide::filter_command::<Command, _>().endpoint(
            move |bot: Bot, msg: Message, cmd: Command| {
                let admin_handle = admin_handle.clone();
                let db = db.clone();

                async move {
                    if !is_admin(&admin_handle, &msg) {
                        bot.send_message(msg.chat.id, "You are not authorized")
                            .await?;

                        return Ok(());
                    }

                    let chat_id = msg.chat.id;

                    if let Err(e) = answer(&bot, msg, cmd, db).await {
                        let root = root_cause(&e).to_string();

                        bot.send_message(
                            chat_id,
                            format!("Internal server error: {e}, caused by: {root}"),
                        )
                        .await?;

                        Err(e)
                    } else {
                        Ok(())
                    }
                }
            },
        );

        let message_handler = Update::filter_message().branch(command_handler);

        Dispatcher::builder(bot, message_handler)
            .build()
            .dispatch()
            .await;
    });
}

fn is_admin(admin_handle: &str, message: &Message) -> bool {
    message
        .from
        .as_ref()
        .and_then(|user| user.username.as_deref())
        .map(|username| username == admin_handle)
        .unwrap_or(false)
}

#[derive(Debug)]
enum AnswerError {
    Tg(teloxide::RequestError),
    Host(ParseHostError),
    Relay(crate::error::Error),
}

impl From<teloxide::RequestError> for AnswerError {
    fn from(value: teloxide::RequestError) -> Self {
        Self::Tg(value)
    }
}

impl From<ParseHostError> for AnswerError {
    fn from(value: ParseHostError) -> Self {
        Self::Host(value)
    }
}

impl From<crate::error::Error> for AnswerError {
    fn from(value: crate::error::Error) -> Self {
        Self::Relay(value)
    }
}

impl std::fmt::Display for AnswerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to respond to message")
    }
}

impl std::error::Error for AnswerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tg(tg) => Some(tg),
            Self::Host(host) => Some(host),
            Self::Relay(relay) => Some(relay),
        }
    }
}

fn root_cause<'a>(
    mut error: &'a (dyn std::error::Error + 'static),
) -> &'a (dyn std::error::Error + 'static) {
    while let Some(source) = error.source() {
        error = source
    }

    error
}

#[tracing::instrument(skip(bot, msg, db))]
async fn answer(bot: &Bot, msg: Message, cmd: Command, db: Db) -> Result<(), AnswerError> {
    match cmd {
        Command::Help | Command::Start => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Block { domain } => {
            let domain = parse_host(&domain)?;

            db.add_blocks(vec![domain.clone()]).await?;

            bot.send_message(msg.chat.id, format!("{domain} has been blocked"))
                .await?;
        }
        Command::Unblock { domain } => {
            let domain = parse_host(&domain)?;

            db.remove_blocks(vec![domain.clone()]).await?;

            bot.send_message(msg.chat.id, format!("{domain} has been unblocked"))
                .await?;
        }
        Command::Allow { domain } => {
            let domain = parse_host(&domain)?;

            db.add_allows(vec![domain.clone()]).await?;

            bot.send_message(msg.chat.id, format!("{domain} has been allowed"))
                .await?;
        }
        Command::Disallow { domain } => {
            let domain = parse_host(&domain)?;

            db.remove_allows(vec![domain.clone()]).await?;

            bot.send_message(msg.chat.id, format!("{domain} has been disallowed"))
                .await?;
        }
        Command::ListAllowed => {
            let allowed = db.allows().await?;

            for chunk in allowed.chunks(50) {
                bot.send_message(msg.chat.id, chunk.join("\n")).await?;
            }
        }
        Command::ListBlocks => {
            let blocks = db.blocks().await?;

            for chunk in blocks.chunks(50) {
                bot.send_message(msg.chat.id, chunk.join("\n")).await?;
            }
        }
        Command::ListConnected => {
            let connected = db.connected_ids().await?;

            for chunk in connected.chunks(50) {
                bot.send_message(msg.chat.id, chunk.join("\n")).await?;
            }
        }
    }

    Ok(())
}
