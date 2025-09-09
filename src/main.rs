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
    schedules: HashMap<String, Schedule>,
}

impl GlobalData {
    async fn new() -> anyhow::Result<GlobalData> {
        let mut schedules: HashMap<String, Schedule> = HashMap::new();
        let mut interval = time::interval(Duration::from_millis(500));

        let groups: Vec<String> = if cfg!(debug_assertions) {
            vec!["ИС501".to_string(), "ИС502".to_string()]
        } else {
            GroupsList::fetch()
                .await?
                .iter()
                .map(|group| group.group_name.clone())
                .collect()
        };

        let groups_with_subgroups: Vec<String> = groups
            .iter()
            .flat_map(|group| {
                (0..3).map(move |subgroup| format!("{}.{}", group, subgroup + 1).to_string())
            })
            .collect();

        for group in groups_with_subgroups {
            interval.tick().await;

            log::debug!("Fetching {} schedule", group);
            let schedule = Schedule::fetch(&group).await;
            schedules.insert(group.clone(), schedule);
        }

        log::info!("Finished fetching schedules for {} groups", schedules.len());

        Ok(GlobalData { schedules })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        // .with_max_level(tracing::Level::DEBUG)
        .init();

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

                let schedule = schedules.get("ИС502.1").unwrap();

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "<i>Расписание для ИС502.1</i>:\n{}",
                        schedule.weeks[schedule.current_week - 1]
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
