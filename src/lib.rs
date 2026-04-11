use std::{collections::HashMap, sync::Arc};

use anyhow::Error;
use steamid_ng::SteamID;
use tokio::sync::RwLock;

use crate::{
    config::Config,
    modules::{tom_react::TomReact, vanity_resolver::VanityResolver},
};

pub mod commands;
pub mod config;
pub mod reports;
pub mod util;
pub mod modules {
    pub mod tom_react;
    pub mod vanity_resolver;
}

pub type Context<'a> = poise::Context<'a, BotData, Error>;

pub struct BotData {
    pub reports: Arc<dyn reports::ReportDB + Send + Sync>,
    pub lists: HashMap<SteamID, Vec<String>>,

    pub react: RwLock<TomReact>,
    pub vanity: VanityResolver,
    pub config: Config,
}