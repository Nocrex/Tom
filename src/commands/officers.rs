use std::time::Duration;

use anyhow::Result;
use poise::{
    CreateReply,
    serenity_prelude::{
        self as serenity, ButtonStyle, Color, ComponentInteraction, CreateActionRow, CreateButton,
        CreateEmbed, CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption, EditThread, ForumTagId, GuildChannel,
    },
};

use crate::{Context, reports::Report, util::GetJumpUrl};

pub(super) fn register() -> Vec<poise::Command<crate::BotData, anyhow::Error>> {
    vec![export(), report()]
}

#[poise::command(slash_command, owners_only)]
async fn export(ctx: Context<'_>) -> Result<()> {
    ctx.defer_ephemeral().await?;
    let now = std::time::Instant::now();
    crate::reports::exports::export(ctx.data().reports.clone(), &ctx.data().config.report.export)
        .await?;

    ctx.say(format!("Done {}s", now.elapsed().as_secs_f32()))
        .await?;
    Ok(())
}

/// Manage the current report
#[poise::command(
    slash_command,
    guild_only,
    check = "super::checks::in_thread",
    check = "super::checks::officer_check"
)]
async fn report(ctx: Context<'_>) -> Result<()> {
    let channel = ctx.guild_channel().await.unwrap();
    let jump_url = channel.jump_url();

    match ctx.data().reports.report(&jump_url, true).await? {
        Some(report) => edit(ctx, report).await,
        None => show_verdict_menu(ctx, channel).await,
    }
}

async fn show_verdict_menu(ctx: Context<'_>, channel: GuildChannel) -> Result<()> {
    let verdict = channel.applied_tags.iter().find_map(|tagid| {
        ctx.data()
            .config
            .report
            .deny_tags
            .iter()
            .find(|t| t.1 == u64::from(*tagid))
    });

    let id = ctx.id();
    let approve_button = format!("{id}_approve");
    let deny_button = format!("{id}_deny");

    let buttons = CreateActionRow::Buttons(if verdict.is_none() {
        vec![
            CreateButton::new(&approve_button)
                .label("Approve")
                .style(ButtonStyle::Success),
            CreateButton::new(&deny_button)
                .label("Deny")
                .style(ButtonStyle::Danger),
        ]
    } else {
        vec![
            CreateButton::new(&approve_button)
                .label("Change verdict")
                .style(ButtonStyle::Primary),
            CreateButton::new(&deny_button)
                .label("Remove verdict")
                .style(ButtonStyle::Secondary),
        ]
    });

    let msg = CreateReply::default()
        .embed(
            CreateEmbed::new()
                .title(format!("Report \"{}\"", channel.name()))
                .field(
                    "Current verdict",
                    verdict.map_or("None", |tag| &tag.0),
                    true,
                )
                .color(if verdict.is_some() {
                    Color::DARK_RED
                } else {
                    Color::BLUE
                }),
        )
        .components(vec![buttons])
        .ephemeral(true);

    let msg_handle = ctx.send(msg).await?;

    if let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&id.to_string()))
        .timeout(std::time::Duration::from_mins(5))
        .await
    {
        if (verdict.is_some() && press.data.custom_id == approve_button)
            || (verdict.is_none() && press.data.custom_id == deny_button)
        {
            deny_verdict(ctx, press, channel).await?;
        } else if verdict.is_some() && press.data.custom_id == deny_button {
            let tags = channel
                .applied_tags
                .iter()
                .filter(|t| !ctx.data().config.report.is_deny_tag((**t).into()))
                .copied();

            ctx.http()
                .edit_thread(
                    channel.id,
                    &EditThread::new().applied_tags(tags),
                    Some("Verdict removed"),
                )
                .await?;

            channel.say(ctx, "Verdict removed").await?;
        }
    }

    let _ = msg_handle.delete(ctx).await;

    Ok(())
}

async fn deny_verdict(
    ctx: Context<'_>,
    press: ComponentInteraction,
    channel: GuildChannel,
) -> Result<()> {
    let selector_id = format!("{}_verdictselector", ctx.id());

    let selector = CreateActionRow::SelectMenu(
        CreateSelectMenu::new(
            &selector_id,
            CreateSelectMenuKind::String {
                options: ctx
                    .data()
                    .config
                    .report
                    .deny_tags
                    .iter()
                    .enumerate()
                    .map(|(i, (n, _))| CreateSelectMenuOption::new(n, i.to_string()))
                    .collect(),
            },
        )
        .min_values(1)
        .max_values(1),
    );

    press
        .create_response(
            ctx.http(),
            serenity::CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().components(vec![selector]),
            ),
        )
        .await?;

    let Some(interact) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |d| d.data.custom_id == selector_id)
        .timeout(Duration::from_secs(20))
        .await
    else {
        return Ok(());
    };

    match &interact.data.kind {
        serenity::ComponentInteractionDataKind::StringSelect { values } => {
            let ind = values.first().unwrap().parse::<usize>().unwrap();

            let tag = ctx.data().config.report.deny_tags.get(ind).unwrap();

            let tags = channel
                .applied_tags
                .iter()
                .filter(|tag| !ctx.data().config.report.is_deny_tag((**tag).into()))
                .copied()
                .chain(std::iter::once(ForumTagId::from(tag.1)));

            ctx.http()
                .edit_thread(
                    channel.id,
                    &EditThread::new().applied_tags(tags),
                    Some("Verdict changed"),
                )
                .await?;

            channel
                .say(ctx, format!("Added mark \"{}\"", tag.0))
                .await?;
        }
        _ => (),
    }

    Ok(())
}

async fn edit(ctx: Context<'_>, report: Report) -> Result<()> {
    /*
     *      show menu with actions
     *      | update last seen | edit | unapprove |
     */
    todo!();
}

async fn approve(ctx: Context<'_>) -> Result<()> {
    /*
     *      parse steamids from message history
     *      send interactable approve message
     *      add player, remove player, edit player
     *      approve button, defer button
     *      send summary message + approve image / button for other officer to approve
     */
    todo!();
}
