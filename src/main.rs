use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{
    self as serenity, ActivityData, CacheHttp, ChannelId, CreateAttachment, CreateMessage
};
use tokio::sync::RwLock;

use tom::{
    BotData,
    commands,
    config::Config,
    util,
    modules::{tom_react::TomReact, vanity_resolver::VanityResolver}, reports::{ReportDB, sql::PostgresDB},
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("tom", log::LevelFilter::Debug)
        .init();
    
    let token = dotenv::var("TOKEN").expect("Missing discord TOKEN");
    let intents =
        serenity::GatewayIntents::non_privileged().union(serenity::GatewayIntents::MESSAGE_CONTENT);

    let config: Config = toml::from_str(&std::fs::read_to_string("config.toml").unwrap()).unwrap();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions::<BotData, anyhow::Error> {
            commands: commands::register(),
            on_error: |e| {
                Box::pin(async move {
                    match e {
                        poise::FrameworkError::EventHandler {
                            error,
                            ctx,
                            framework,
                            ..
                        } => {
                            log::error!("{error:?}");
                            ChannelId::from(framework.user_data.config.error_channel)
                                .send_files(
                                    &ctx,
                                    vec![CreateAttachment::bytes(
                                        format!("{error:?}"),
                                        "error.txt",
                                    )],
                                    CreateMessage::new(),
                                )
                                .await
                                .expect("Could not send error message");
                        }
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            log::error!("{error:?}");
                            ChannelId::from(ctx.data().config.error_channel)
                                .send_files(
                                    &ctx,
                                    vec![CreateAttachment::bytes(
                                        format!("{error:?}"),
                                        "error.txt",
                                    )],
                                    CreateMessage::new(),
                                )
                                .await
                                .expect("Could not send error message");
                        }
                        poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
                            log::error!("Code panicked! {payload:?}");
                            ChannelId::from(ctx.data().config.error_channel)
                                .send_files(
                                    &ctx,
                                    vec![CreateAttachment::bytes(
                                        format!("Code panicked!\n{:?}", payload),
                                        "panic.txt",
                                    )],
                                    CreateMessage::new(),
                                )
                                .await
                                .expect("Could not send error message");
                        }
                        _ => (),
                    }
                })
            },
            pre_command: |ctx| {
                Box::pin(async move {
                    log::info!(
                        "{} used command {}",
                        ctx.author().display_name(),
                        ctx.invocation_string()
                    );
                })
            },
            event_handler: |ctx, ev, _fctx, data| {
                Box::pin(async move {
                    match ev {
                        serenity::FullEvent::Message { new_message } => {
                            if data
                                .react
                                .read()
                                .await
                                .should_interact(new_message, &ctx)
                                .await?
                            {
                                data.react
                                    .write()
                                    .await
                                    .on_message(new_message, &ctx)
                                    .await?;
                            }
                            data.vanity.on_message(new_message, &ctx, data).await?;
                        }
                        _ => (),
                    }
                    Ok(())
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let react = TomReact::load(ctx.http(), config.react.clone()).await?;
                let vanity = VanityResolver::new(config.vanity.clone());
                let lists = util::load_lists(&config.report.ext_list_dir)?;
                
                let reports = Arc::new(PostgresDB::new(&dotenv::var("DATABASE_URL").expect("Missing DATABASE_URL")).await?);
                
                {
                    let ctx = ctx.clone();
                    let reports = reports.clone();
                    tokio::task::spawn(async move {
                        loop {
                            if let Ok(count) = reports.reported_count().await {
                                ctx.set_presence(Some(ActivityData::watching(format!("{count} SteamIDs"))), serenity::OnlineStatus::Online);
                            } else {
                                log::warn!("Error while fetching report count");
                            }
                            tokio::time::sleep(Duration::from_mins(1)).await;
                        }
                    });
                }

                log::info!("Bot up and running");
                Ok(BotData {
                    reports,
                    lists,
                    config,
                    react: RwLock::new(react),
                    vanity,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
