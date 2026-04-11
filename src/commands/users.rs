use crate::{
    BotData, Context, util::{self, SteamIDProfileLink}
};
use anyhow::Result;
use itertools::Itertools;
use poise::{
    CreateReply,
    serenity_prelude::{self as serenity, Color, CreateEmbed, CreateEmbedAuthor},
};

pub(super) fn register() -> Vec<poise::Command<BotData, anyhow::Error>> {
    vec![points(), lookup(), toplist()]
}

/// Gets a users (or your own) report point count
#[poise::command(slash_command, guild_only, user_cooldown = 5)]
async fn points(
    ctx: Context<'_>,
    #[description = "User to get point count for"] user: Option<serenity::User>,
) -> Result<()> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let privileged = super::checks::is_officer(ctx).await?;

    let Some((reporter, points)) = ctx
        .data()
        .reports
        .reporter_with_points(user.id, privileged)
        .await?
    else {
        ctx.send(
            CreateReply::default()
                .content("User does not have any reports")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };
    let mut embed = CreateEmbed::new()
        .author(
            CreateEmbedAuthor::new(user.display_name()).icon_url(
                user.avatar_url()
                    .unwrap_or_else(|| user.default_avatar_url()),
            ),
        )
        .field("Report Count", points.to_string(), false);

    if privileged {
        let profile_link = reporter
            .steamid
            .as_ref()
            .map_or_else(|| "not on record".to_string(), |sid| sid.profile());

        let recentreports = reporter
            .reports
            .iter()
            .take(5)
            .map(|r| &r.thread_url)
            .join("\n");

        embed = embed.field("Steam profile", profile_link, false).field(
            "Recent reports",
            recentreports,
            false,
        );
    }
    ctx.send(CreateReply::default().embed(embed).ephemeral(true))
        .await?;
    Ok(())
}

/// List the top 20 people based on report count
#[poise::command(slash_command, guild_only, user_cooldown = 5)]
async fn toplist(ctx: Context<'_>) -> Result<()> {
    let mut reporters = ctx.data().reports.reporters_with_points().await?;

    reporters.sort_by_key(|(_, p)| *p);
    let msg = reporters
        .iter()
        .rev()
        .take(20)
        .map(|(r, p)| format!("{p}: {}", r.id))
        .join("\n");

    ctx.send(
        CreateReply::default()
            .embed(CreateEmbed::new().title("Top Reporters").description(msg))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Look up previous reports of a SteamID
#[poise::command(slash_command, guild_only, user_cooldown = 2)]
async fn lookup(ctx: Context<'_>, steamid: String) -> Result<()> {
    let Ok(id) = util::get_steamid(&steamid).await else {
        ctx.send(
            CreateReply::default()
                .content("Could not resolve steam id")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let mut embed = CreateEmbed::new().title(format!("Information for {}", id.steam64()));

    let reports = ctx.data().reports.find_reports(&id).await?;
    if !reports.is_empty() {
        let report_msg = reports
            .iter()
            .map(|report| {
                format!(
                    "{} [{}]{}",
                    report.thread_url,
                    report.attribute,
                    if !report.verified {
                        " -- (unverified)"
                    } else {
                        ""
                    },
                )
            })
            .join("\n");

        embed = embed.field("Reports", report_msg, false);
    }

    let lists = ctx.data().lists.get(&id);

    if let Some(lists) = lists {
        let lists_msg = lists.iter().join("\n");
        embed = embed.field("External lists", lists_msg, false);
    }

    embed = embed.color(match (!reports.is_empty(), lists.is_some()) {
        (true, _) => Color::ORANGE,
        (false, true) => Color::new(0xFEE75C),
        (false, false) => Color::BLUE,
    });

    if reports.is_empty() && lists.is_none() {
        embed = embed.field("No reports found", "", false);
    }

    ctx.send(CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}
