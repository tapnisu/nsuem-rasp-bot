use std::{collections::HashMap, error::Error, process::exit, sync::Arc, time::Duration};

use nsuem_rasp_bot::{Schedule, lists::groups::GroupsList};
use teloxide::{
    prelude::*,
    types::{Me, ParseMode},
    utils::command::BotCommands,
};
use tokio::{sync::RwLock, time};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct GlobalData {
    schedules: HashMap<String, String>,
}

impl GlobalData {
    async fn new() -> anyhow::Result<GlobalData> {
        let mut schedules: HashMap<String, String> = HashMap::new();
        let mut interval = time::interval(Duration::from_millis(500));

        for group in GroupsList::fetch().await? {
            interval.tick().await;

            log::debug!("Fetching {} schedule", group.group_name);
            let schedule = Schedule::fetch(&group.group_name).await.to_string();
            schedules.insert(group.group_name.clone(), schedule);
        }

        log::info!("Finished fetching schedules for {} groups", schedules.len());

        Ok(GlobalData { schedules })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    dotenvy::dotenv().ok();

    let bot = Bot::from_env();
    let global_data = match GlobalData::new().await {
        Ok(data) => Arc::new(RwLock::new(data)),
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    let mut interval = time::interval(Duration::from_secs(60 * 60));
    tokio::spawn({
        let global_data = global_data.clone();

        async move {
            loop {
                interval.tick().await;
                let mut rw_data = global_data.write().await;

                match GlobalData::new().await {
                    Ok(data) => *rw_data = data,
                    Err(err) => eprintln!("{err}"),
                }
            }
        }
    });

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![global_data])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "Поддерживаются следующие команды:"
)]
enum Command {
    #[command(description = "отображает этот текст.")]
    Start,
    #[command(description = "отображает расписание для ИС502.")]
    Rasp,
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
    global_data: Arc<RwLock<GlobalData>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Rasp) => {
                let schedules = {
                    let data = global_data.read().await;
                    data.schedules.clone()
                };

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "<i>Расписание для ИС502.1</i>:\n{}",
                        schedules.get("%D0%98%D0%A1502").unwrap()
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Команда не найдена!").await?;
            }
        }
    }

    Ok(())
}

async fn callback_handler(
    _bot: Bot,
    _q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    Ok(())
}
