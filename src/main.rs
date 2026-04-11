use std::collections::HashMap;

use anyhow::Error;
use poise::serenity_prelude::{
    self as serenity, CacheHttp, ChannelId, CreateAttachment, CreateMessage,
};
use steamid_ng::SteamID;

use crate::{
    config::Config,
    modules::{tom_react::TomReact, vanity_resolver::VanityResolver},
};

mod commands;
mod config;
mod reports;
mod util;
mod modules {
    pub(crate) mod tom_react;
    pub(crate) mod vanity_resolver;
}

type Context<'a> = poise::Context<'a, BotData, Error>;

struct BotData {
    reports: Box<dyn reports::ReportDB + Send + Sync>,
    lists: HashMap<SteamID, Vec<String>>,

    react: TomReact,
    vanity: VanityResolver,
    config: Config,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    env_logger::init();
    let token = dotenv::var("TOKEN").expect("Missing discord TOKEN");
    let intents =
        serenity::GatewayIntents::non_privileged().union(serenity::GatewayIntents::MESSAGE_CONTENT);

    let config: Config =
        serde_json::from_reader(std::fs::File::open("config.toml").unwrap()).unwrap();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::users::lookup(),
                commands::users::points(),
                commands::users::toplist(),
            ],
            on_error: |e| {
                Box::pin(async move {
                    match e {
                        poise::FrameworkError::EventHandler {
                            error,
                            ctx,
                            framework,
                            ..
                        } => {
                            ChannelId::from(framework.user_data.config.error_channel)
                                .send_files(
                                    &ctx,
                                    vec![CreateAttachment::bytes(
                                        format!("{error:#?}"),
                                        "error.txt",
                                    )],
                                    CreateMessage::new(),
                                )
                                .await;
                        }
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            ChannelId::from(ctx.data().config.error_channel)
                                .send_files(
                                    &ctx,
                                    vec![CreateAttachment::bytes(
                                        format!("{error:#?}"),
                                        "error.txt",
                                    )],
                                    CreateMessage::new(),
                                )
                                .await;
                        }
                        poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
                            ChannelId::from(ctx.data().config.error_channel)
                                .send_files(
                                    &ctx,
                                    vec![CreateAttachment::bytes(
                                        format!("Code panicked!\n{:#?}", payload),
                                        "panic.txt",
                                    )],
                                    CreateMessage::new(),
                                )
                                .await;
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
            event_handler: |ctx, ev, fctx, data| {
                Box::pin(async move {
                    match ev {
                        serenity::FullEvent::Message { new_message } => {
                            data.react.on_message(new_message, &ctx).await?;
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
                let react = TomReact::load(ctx.http(), config.react).await?;
                let vanity = VanityResolver::new(config.vanity.clone());
                let lists = util::load_lists(&config.report.ext_list_dir)?;

                log::info!("Bot up and running");
                Ok(BotData {
                    lists,
                    config,
                    react,
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
