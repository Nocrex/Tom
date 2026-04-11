use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use itertools::Itertools;
use poise::serenity_prelude::{
    CacheHttp, CreateAllowedMentions, CreateEmbed, CreateMessage, Message,
};
use steamid_ng::SteamID;

use crate::{
    BotData,
    config::VanityConfig,
    util::{self, PERM_LINK_PATTERN, SteamIDProfileLink, VANITY_LINK_PATTERN},
};

pub(crate) struct VanityResolver {
    cfg: VanityConfig,
}

impl VanityResolver {
    pub fn new(cfg: VanityConfig) -> Self {
        Self { cfg }
    }

    pub async fn on_message(
        &self,
        msg: &Message,
        http: &impl CacheHttp,
        data: &BotData,
    ) -> Result<()> {
        if msg.author.bot {
            return Ok(());
        }

        let Some(chan) = msg.channel(http).await?.guild() else {
            return Ok(());
        };

        let channel_id = chan.parent_id.unwrap_or(chan.id);

        if !self.cfg.resolve_channels.contains(&u64::from(channel_id)) {
            return Ok(());
        }


        let mut resolved_steamids = HashMap::new();
        let mut unresolved_steamids = vec![];

        let vanity_links = VANITY_LINK_PATTERN
            .captures_iter(&msg.content)
            .map(|m| (m.get(0).unwrap().as_str(), m.get(1).unwrap().as_str()))
            .collect::<Vec<_>>();
        for (url, name) in vanity_links.into_iter().dedup() {
            match util::resolve_vanity(url).await? {
                Some(id) => {
                    resolved_steamids.insert(name, id);
                }
                None => {
                    unresolved_steamids.push(name);
                }
            }
        }

        let mut steamids = PERM_LINK_PATTERN
            .captures_iter(&msg.content)
            .map(|id| {
                SteamID::from_str(id.get(1).ok_or(anyhow::anyhow!("Capture index"))?.as_str())
                    .map_err(|e| anyhow::Error::from(e))
            })
            .collect::<Result<Vec<SteamID>>>()?;

        steamids.extend(resolved_steamids.values().cloned());

        let mut reports = HashMap::new();
        let mut list_marks = HashMap::new();

        for id in &steamids {
            let reps = data.reports.find_reports(id).await?;
            if let Some(r) = reps.iter().find(|r| r.verified).or_else(|| reps.first()) {
                reports.insert(id.clone(), r.clone());
            }

            if let Some(lists) = data.lists.get(id) {
                list_marks.insert(id.clone(), lists);
            }
        }

        if !resolved_steamids.is_empty()
            || !unresolved_steamids.is_empty()
            || !reports.is_empty()
            || !list_marks.is_empty()
        {
            let mut embed = CreateEmbed::new();
            if !resolved_steamids.is_empty() {
                let links = resolved_steamids
                    .iter()
                    .map(|(url, id)| format!("\"{url}\": {}", id.profile()))
                    .join("\n");
                embed = embed.field("Permanent Links", links, false);
            }

            if !unresolved_steamids.is_empty() {
                let list = unresolved_steamids.join("\n");
                embed = embed.field("Could not find profiles for", list, false);
            }

            if !reports.is_empty() {
                let reps = reports
                    .iter()
                    .map(|(id, r)| {
                        format!(
                            "`{}` -> {}{}",
                            id.steam64(),
                            r.thread_url,
                            if !r.verified { " (unverified)" } else { "" }
                        )
                    })
                    .join("\n");

                embed = embed.field("Reports", reps, false);
            } else {
                embed = embed.field("SteamIDs have not been reported", "", false);
            }

            if !list_marks.is_empty() {
                let marks = list_marks
                    .iter()
                    .map(|(id, l)| format!("`{}` -> {}", id.steam64(), l.iter().join(", ")))
                    .join("\n");
                embed = embed.field("Players present in lists", marks, false);
            }

            msg.channel_id
                .send_message(
                    http,
                    CreateMessage::new()
                        .reference_message(msg)
                        .embed(embed)
                        .allowed_mentions(CreateAllowedMentions::new().replied_user(false)),
                )
                .await?;
        }

        Ok(())
    }
}
