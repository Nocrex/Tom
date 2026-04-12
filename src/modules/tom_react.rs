use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use poise::serenity_prelude::{
    Attachment, CacheHttp, ChannelId, CreateMessage, Http, Message, UserId, futures::StreamExt,
};
use rand::RngExt;
use tokio::time::Instant;

use crate::config::ReactConfig;

pub struct TomReact {
    cooldowns: HashMap<UserId, Instant>,
    images: Vec<Attachment>,
    config: ReactConfig,
}

impl TomReact {
    pub async fn load(http: &Http, config: ReactConfig) -> Result<Self> {
        let images = ChannelId::from(config.image_channel)
            .messages_iter(&http)
            .filter_map(async |msg| msg.ok().map(|m| m.attachments))
            .collect::<Vec<Vec<Attachment>>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        tracing::info!("Loaded {} reaction images", images.len());

        Ok(Self {
            cooldowns: Default::default(),
            images,
            config,
        })
    }
    
    pub async fn should_interact(&self, msg: &Message, http: &impl CacheHttp) -> Result<bool> {
        if u64::from(msg.channel_id) == self.config.image_channel || msg.mentions_me(http).await? {
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn on_message(&mut self, msg: &Message, http: &impl CacheHttp) -> Result<()> {
        if u64::from(msg.channel_id) == self.config.image_channel {
            self.images.extend(msg.attachments.iter().cloned());
            return Ok(());
        }
        if !msg.mentions_me(http).await? {
            return Ok(());
        }
        let now = Instant::now();
        self.cooldowns.retain(|_, v| *v > now);

        if self.cooldowns.contains_key(&msg.author.id) || self.images.is_empty() {
            msg.react(http, '\u{1F4A4}').await?;
            Ok(())
        } else {
            self.cooldowns.insert(
                msg.author.id,
                now + Duration::from_secs(self.config.cooldown_seconds),
            );
            let img_ind = rand::rng().random_range(0..self.images.len());
            msg.channel_id
                .send_message(
                    http,
                    CreateMessage::default()
                        .content(&self.images[img_ind].url)
                        .reactions(['\u{2764}']),
                )
                .await?;
            Ok(())
        }
    }
}
