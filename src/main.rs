use std::{env, error::Error, process::exit, sync::Arc, time::Duration};

use futures::future::join_all;
use nsuem_rasp_bot::{
    Schedule, ScheduleCache, lists::groups::GroupsList, utils::cyrillic::ToCyrillic,
};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use teloxide::{
    prelude::*,
    types::{
        InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputMessageContentText,
        Me, ParseMode,
    },
    utils::command::BotCommands,
};
use tokio::{sync::RwLock, time};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct GlobalData {
    groups: Vec<String>,
    groups_with_subgroups: Vec<String>,
}

impl GlobalData {
    async fn new(bot: &Bot, pool: &Pool<Sqlite>) -> anyhow::Result<GlobalData> {
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

        log::info!(
            "started fetching schedules for {} groups",
            groups_with_subgroups.len()
        );
        log::debug!("fetching for {:?}", groups_with_subgroups);

        for group in groups_with_subgroups.clone() {
            interval.tick().await;

            log::debug!("fetching {} schedule", group);
            let schedule = Schedule::fetch(&group).await;

            if let Some(old_schedule) = Schedule::fetch_cached(&group, pool).await?
                && let Some(schedule_diff) = schedule.find_diff(&old_schedule)
            {
                bot.send_message(ChatId(1104237221), schedule_diff)
                    .parse_mode(ParseMode::Html)
                    .await
                    .ok();
            }

            schedule.write_to_cache(&group, pool).await?;
        }

        log::debug!("fetched for {:?} subgroups", groups_with_subgroups);

        Ok(GlobalData {
            groups,
            groups_with_subgroups,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    log::info!("starting");

    dotenvy::dotenv().ok();

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let bot = Bot::from_env();
    let me = bot.get_me().await?;
    log::debug!("bot info: {:?}", me);

    let global_data = match GlobalData::new(&bot.clone(), &pool).await {
        Ok(data) => Arc::new(RwLock::new(data)),
        Err(err) => {
            log::error!("{err}");
            exit(1);
        }
    };

    let mut interval = time::interval(Duration::from_secs(60 * 60));
    interval.tick().await;

    tokio::spawn({
        let global_data = global_data.clone();
        let updater_bot = bot.clone();
        let pool = pool.clone();

        async move {
            loop {
                interval.tick().await;
                let mut rw_data = global_data.write().await;

                match GlobalData::new(&updater_bot, &pool).await {
                    Ok(data) => *rw_data = data,
                    Err(err) => log::error!("{err}"),
                }
            }
        }
    });

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_inline_query().endpoint(inline_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![global_data, pool])
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
    pool: Pool<Sqlite>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Rasp) => {
                let schedule = Schedule::fetch_cached("ИС502.1", &pool).await?.unwrap();

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "<i>Расписание для ИС502.1</i>:\n\n{}",
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

async fn inline_handler(
    bot: Bot,
    q: InlineQuery,
    global_data: Arc<RwLock<GlobalData>>,
    pool: Pool<Sqlite>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let query_lower = q.query.to_uppercase().to_cyrillic();
    log::debug!("received query {}", query_lower);

    let groups_with_subgroups = {
        let data = global_data.read().await;
        data.groups_with_subgroups.clone()
    };

    log::debug!("got groups {:?}", groups_with_subgroups);

    let candidates: Vec<String> = if query_lower.is_empty() {
        groups_with_subgroups
    } else {
        groups_with_subgroups
            .iter()
            .filter(|g| g.to_uppercase().contains(&query_lower))
            .cloned()
            .collect()
    };

    log::debug!("got candidates {:?}", candidates);

    let futures = candidates.into_iter().map(|group| {
        let pool = pool.clone();

        async move {
            Schedule::fetch_cached(&group, &pool)
                .await
                .ok()
                .flatten()
                .map(|schedule| {
                    let text = format!(
                        "<i>Расписание для {} на сегодня</i>:\n\n{}",
                        group,
                        schedule.weeks[schedule.current_week - 1].days[schedule.today_day_id]
                            .clone()
                            .unwrap()
                    );

                    InlineQueryResultArticle::new(
                        uuid::Uuid::new_v4().to_string(),
                        group,
                        InputMessageContent::Text(
                            InputMessageContentText::new(text).parse_mode(ParseMode::Html),
                        ),
                    )
                    .into()
                })
        }
    });

    let results: Vec<InlineQueryResult> = join_all(futures).await.into_iter().flatten().collect();

    log::debug!("got results {:?}", results);

    bot.answer_inline_query(q.id, results).cache_time(0).await?;

    Ok(())
}
