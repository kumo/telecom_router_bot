use dotenv;
use std::env;
use std::error::Error;
use teloxide::{prelude2::*, utils::command::BotCommand};
use tokio::time::{sleep, Duration};

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

extern crate callog_bot;
use callog_bot::timm;
use callog_bot::timm::{calls::PhoneCall, stats::LineSpeed};

//mod timm;
//use timm::PhoneCall;

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display today's calls.")]
    Today,
    #[command(description = "display recent calls.")]
    Recent,
    #[command(description = "display all calls.")]
    All,
    #[command(description = "display current speed.")]
    Speed,
    #[command(description = "reboot the modem.")]
    Reboot,
}

async fn list_all_calls(bot: AutoSend<Bot>, chat_id: i64) {
    if let Some(mut phone_calls) = timm::calls::download_calls().await {
        if phone_calls.is_empty() {
            if let Err(_) = bot
                .send_message(
                    chat_id,
                    "There are no recent calls in memory -- was the modem recently rebooted?",
                )
                .await
            {
                warn!("Couldn't send list_all_calls message.");
            }
        } else {
            debug!("There are new calls");

            phone_calls.reverse();
            for phone_call in &phone_calls {
                debug!("{}", phone_call);

                if let Err(_) = bot.send_message(chat_id, format!("{}", phone_call)).await {
                    warn!("Couldn't send list_all_calls message.");
                }
            }

            debug!("There are {} phone calls.", phone_calls.len());
        }
    } else {
        debug!("There might be no phone calls in memory.");

        if let Err(_) = bot
            .send_message(chat_id, "Problem getting latest calls!")
            .await
        {
            warn!("Couldn't send list_all_calls message.");
        }
    }
}

async fn list_recent_calls(bot: AutoSend<Bot>, chat_id: i64) {
    let mut recent_phone_calls: Vec<PhoneCall> = timm::calls::download_calls()
        .await
        .unwrap_or(Vec::new())
        .into_iter()
        .filter(|phone_call| phone_call.is_today())
        .collect();

    debug!("There are {} recent phone calls.", recent_phone_calls.len());

    if recent_phone_calls.is_empty() {
        if let Err(_) = bot
            .send_message(
                chat_id,
                "There are no recent calls in memory -- was the modem recently rebooted?",
            )
            .await
        {
            warn!("Couldn't send list_recent_calls message.");
        }
    } else {
        recent_phone_calls.reverse();
        for phone_call in &recent_phone_calls {
            debug!("{}", phone_call);

            if let Err(_) = bot.send_message(chat_id, format!("{}", phone_call)).await {
                warn!("Couldn't send list_recent_calls message.");
            }
        }
    }
}

async fn monitor_calls(bot: AutoSend<Bot>, chat_id: i64) {
    info!("Starting - monitor_calls");

    let mut last_call: Option<PhoneCall> = None;

    loop {
        info!("Checking calls");

        let latest_calls = timm::calls::download_calls()
            .await
            .and_then(|calls| timm::calls::get_new_calls(&last_call, calls));

        if let Some(mut latest_calls) = latest_calls {
            debug!("There are new calls");

            latest_calls.reverse();
            for phone_call in &latest_calls {
                debug!("{}", phone_call);

                if let Err(_) = bot.send_message(chat_id, format!("{}", phone_call)).await {
                    warn!("Couldn't send monitor_calls message.");
                }
            }

            if let Some(call) = Some(latest_calls.last().cloned()) {
                last_call = call;
            }
        } else {
            warn!("No calls found.")
        }

        sleep(Duration::from_secs(60)).await;
    }
}

async fn monitor_speed(bot: AutoSend<Bot>, chat_id: i64) {
    info!("Starting - monitor_speed");

    let mut last_speed = LineSpeed::Normal;
    let mut last_ip = String::new();

    loop {
        info!("Checking stats");

        if let Some(stats) = timm::stats::download_stats().await {
            if stats.speed != last_speed {
                if let Err(_) = bot.send_message(chat_id, format!("{}", stats.speed)).await {
                    warn!("Couldn't send monitor_speed (speed) message.");
                }

                debug!("{}", stats.speed);
                last_speed = stats.speed;
            } else {
                debug!("Skipping same speed state");
            }

            // NOTE: I am only adding this temporarily, because I want to see how often the IP address
            // changes throughout the day, and perhaps the connection is automatically resetting
            if stats.ip != last_ip {
                if let Err(_) = bot
                    .send_message(chat_id, format!("IP is {}", stats.ip))
                    .await
                {
                    warn!("Couldn't send monitor_speed (ip) message.");
                }

                debug!("{}", stats.ip);
                last_ip = stats.ip;
            } else {
                debug!("Skipping same ip");
            }
        } else {
            warn!("Problem getting stats");
        }

        sleep(Duration::from_secs(5 * 60)).await;
    }
}

async fn list_speed(bot: AutoSend<Bot>, chat_id: i64) {
    if let Some(stats) = timm::stats::download_stats().await {
        if let Err(_) = bot.send_message(chat_id, format!("{}", stats)).await {
            warn!("Couldn't send list_speed message.");
        }
    } else {
        warn!("Problem getting stats");
    }
}

async fn reboot(bot: AutoSend<Bot>, chat_id: i64) {
    if let Some(_) = timm::tools::reboot().await {
        if let Err(_) = bot
            .send_message(chat_id, "The modem should be rebooting.")
            .await
        {
            warn!("Couldn't send should reboot message.");
        }
    } else {
        if let Err(_) = bot
            .send_message(chat_id, "The modem might be rebooting.")
            .await
        {
            warn!("Couldn't send might reboot message.");
        }
    }
}

async fn answer(
    bot: AutoSend<Bot>,
    message: Message,
    command: Command,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id: i64 = env::var("CHAT_ID").expect("CHAT_ID must be set").parse()?;

    if message.chat.id != chat_id {
        bot.send_message(message.chat.id, "I shouldn't speak to strangers.")
            .await?;
        debug!("I shouldn't talk to strangers: {}", message.chat.id);

        return Ok(());
    }

    match command {
        Command::Help => {
            if let Err(_) = bot.send_message(chat_id, Command::descriptions()).await {
                warn!("Couldn't send answer message.");
            }
        }
        Command::Today => {
            list_recent_calls(bot.clone(), chat_id).await;
        }
        Command::Recent => {
            list_recent_calls(bot.clone(), chat_id).await;
        }
        Command::All => {
            list_all_calls(bot.clone(), chat_id).await;
        }
        Command::Speed => {
            list_speed(bot.clone(), chat_id).await;
        }
        Command::Reboot => {
            reboot(bot.clone(), chat_id).await;
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    dotenv::dotenv().ok();

    let chat_id: i64 = env::var("CHAT_ID").expect("CHAT_ID must be set").parse()?;

    let bot = Bot::from_env().auto_send();
    let bot_clone = bot.clone();
    let handler = teloxide::repls2::commands_repl(bot.clone(), answer, Command::ty());

    let tasks = vec![
        tokio::spawn(async move { monitor_calls(bot, chat_id).await }),
        tokio::spawn(async move { monitor_speed(bot_clone, chat_id).await }),
        tokio::spawn(async move { handler.await }),
    ];

    futures::future::join_all(tasks).await;

    Ok(())
}
