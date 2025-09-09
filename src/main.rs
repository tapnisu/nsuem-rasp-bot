use std::{error::Error, process::exit, sync::Arc, time::Duration};

use nsuem_rasp_bot::Schedule;
use teloxide::{
    prelude::*,
    types::{Me, ParseMode},
    utils::command::BotCommands,
};
use tokio::{sync::RwLock, time};

#[derive(Clone, Debug)]
struct GlobalData {
    rasp1: String,
    rasp2: String,
}

impl GlobalData {
    async fn new() -> anyhow::Result<GlobalData> {
        let schedule1 = Schedule::new("%D0%98%D0%A1502/1").await;
        let rasp1 = schedule1.weeks[schedule1.current_week - 1].to_string();

        let schedule2 = Schedule::new("%D0%98%D0%A1502/2").await;
        let rasp2 = schedule2.weeks[schedule2.current_week - 1].to_string();

        Ok(GlobalData { rasp1, rasp2 })
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

    let mut interval = time::interval(Duration::from_secs(5 * 60));
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
    #[command(description = "отображает расписание для ИС502.1.")]
    Rasp1,
    #[command(description = "отображает расписание для ИС502.2.")]
    Rasp2,
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
            Ok(Command::Rasp1) => {
                let rasp1 = {
                    let data = global_data.read().await;
                    data.rasp1.clone()
                };

                bot.send_message(
                    msg.chat.id,
                    format!("<i>Расписание для ИС502.1</i>:\n{}", rasp1),
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
            Ok(Command::Rasp2) => {
                let rasp2 = {
                    let data = global_data.read().await;
                    data.rasp2.clone()
                };

                bot.send_message(
                    msg.chat.id,
                    format!("<i>Расписание для ИС502.2</i>:\n{}", rasp2),
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
