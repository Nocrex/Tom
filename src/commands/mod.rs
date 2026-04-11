use crate::BotData;

mod users;
mod officers;

pub fn register() -> Vec<poise::Command<BotData, anyhow::Error>> {
    vec![
        users::register(),
        officers::register(),
    ].into_iter().flatten().collect()
}

mod checks {
    use anyhow::Result;
    use poise::CreateReply;
    use crate::Context;
    pub(super) fn is_officer(ctx: Context<'_>) -> bool {
        ctx.data().config.report.officer_roles.contains(&u64::from(ctx.author().id))
    }
    
    pub(super) async fn officer_check(ctx: Context<'_>) -> Result<bool> {
        if !is_officer(ctx){
            ctx.send(CreateReply::default().content("You are not allowed to use this command").ephemeral(true)).await?;
            Ok(false)
        }else{
            Ok(true)
        }
    }
    
    pub(super) async fn in_thread(ctx: Context<'_>) -> Result<bool> {
        if !ctx.guild_channel().await.is_some_and(|c|c.thread_metadata.is_some()) {
            ctx.send(CreateReply::default().content("Cannot use this command outside of a thread").ephemeral(true)).await?;
            return Ok(false);
        }
        Ok(true)
    }
}