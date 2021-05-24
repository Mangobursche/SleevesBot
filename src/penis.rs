use rand::Rng;
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::Message,
        guild::Member,
        id::{ChannelId, UserId},
        misc::Mentionable,
    },
};

#[group]
#[commands(penis, duel)]
struct Penis;

#[command]
#[description = "Calculates your or anothers penis size"]
#[usage = "(@user)"]
async fn penis(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if let Ok(user) = args.find::<UserId>() {
        msg.reply(&ctx.http, get_penis(&user).await).await?;
    } else {
        msg.reply(&ctx.http, get_penis(&msg.author.id).await)
            .await?;
    }

    Ok(())
}

#[command]
#[description = "Compares your penis with somebody elses"]
#[usage = "<@user>"]
async fn duel(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let user_id = args.find::<UserId>()?;

    if user_id.0 == 246684413075652612 {
        msg.reply(&ctx.http, "You dare challenge Biggus Dickus?! https://i.kym-cdn.com/entries/icons/original/000/036/026/biggus.jpg").await?;
    } else if rand::thread_rng().gen_bool(0.5) {
        msg.reply(
            &ctx,
            format!(
                "{}'s penis is bigger than {}'s",
                user_id.mention(),
                msg.author.mention()
            ),
        )
        .await?;
    } else {
        msg.reply(
            &ctx,
            format!(
                "{}'s penis is bigger than {}'s",
                msg.author.mention(),
                user_id.mention()
            ),
        )
        .await?;
    }

    Ok(())
}

async fn get_penis(user_id: &UserId) -> String {
    let size: i32 = rand::thread_rng().gen_range(1..15);

    if size < 2 {
        format!(
            "{} ERROR: can not compute measurements less than 1 inch",
            user_id.mention()
        )
    } else {
        format!("{}'s penis is {} inches", user_id.mention(), size)
    }
}

pub async fn guild_member_addition(ctx: &Context, new_member: &Member) {
    ChannelId(703199261776543756)
        .say(&ctx.http, get_penis(&new_member.user.id).await)
        .await
        .unwrap();
}
