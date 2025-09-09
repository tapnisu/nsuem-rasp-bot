use std::{error::Error, process::exit, sync::Arc, time::Duration};

use scraper::{Html, Selector};
use teloxide::{prelude::*, types::Me, utils::command::BotCommands};
use tokio::{sync::RwLock, time};

async fn render_rasp(group_name: &str) -> String {
    let url = format!("https://rasps.nsuem.ru/group/{}", group_name);
    let req = reqwest::get(url).await.unwrap().text().await.unwrap();

    let document = Html::parse_document(&req);
    let row_selector = Selector::parse("table.table tr").unwrap();
    let day_selector = Selector::parse("td.day-header").unwrap();
    let time_selector = Selector::parse("td .extend_time").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let info_selector = Selector::parse(".mainScheduleInfo").unwrap();
    let teacher_selector = Selector::parse(".Teacher a").unwrap();
    let type_selector = Selector::parse(".small.text-muted").unwrap();

    let week_cell = match document
        .select(&Selector::parse("td#blink").unwrap())
        .next()
        .map(|cell| cell.text().collect::<String>().trim().to_string())
        .as_deref()
    {
        Some("Вторая неделя") => 3,
        Some("Первая неделя") | None | Some(_) => 2,
    };

    let mut output: Vec<String> = vec![];

    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&td_selector).collect();

        if cells.len() < week_cell + 1 {
            continue;
        }

        if let Some(day_cell) = row.select(&day_selector).next() {
            let current_day = day_cell.text().collect::<String>().trim().to_string();
            output.push(format!("{}:", current_day));
        }

        let time = row
            .select(&time_selector)
            .next()
            .map(|t| {
                t.text()
                    .collect::<String>()
                    .trim()
                    .replace("--", "-")
                    .to_string()
            })
            .unwrap_or_default();

        let cell = &cells[week_cell];
        if cell.select(&info_selector).next().is_none() {
            continue;
        }

        let subject = cell
            .select(&info_selector)
            .next()
            .map(|c| {
                c.text()
                    .collect::<String>()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .unwrap_or_default();

        let lesson_type = cell
            .select(&type_selector)
            .next()
            .map(|t| t.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let teacher = cell
            .select(&teacher_selector)
            .next()
            .map(|a| a.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        output.push(format!(
            "{} | {} | {} | {}",
            time, subject, teacher, lesson_type
        ));
    }

    output.join("\n")
}

#[derive(Clone, Debug)]
struct GlobalData {
    rasp1: String,
    rasp2: String,
}

impl GlobalData {
    async fn new() -> anyhow::Result<GlobalData> {
        let rasp1 = render_rasp("%D0%98%D0%A1502/1").await;
        let rasp2 = render_rasp("%D0%98%D0%A1502/2").await;

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

                bot.send_message(msg.chat.id, format!("Расписание для ИС502.1:\n\n{}", rasp1))
                    .await?;
            }
            Ok(Command::Rasp2) => {
                let rasp2 = {
                    let data = global_data.read().await;
                    data.rasp2.clone()
                };

                bot.send_message(msg.chat.id, format!("Расписание для ИС502.2:\n\n{}", rasp2))
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
