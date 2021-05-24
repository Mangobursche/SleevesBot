use chrono::Utc;
use serenity::{
    client::Context,
    framework::standard::{
        help_commands,
        macros::{command, group, help},
        Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::{channel::Message, id::UserId},
};
use std::collections::HashSet;

#[group]
#[commands(ping)]
struct General;

#[command]
#[description = "Calculates the delay in ms"]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let delay = Utc::now() - msg.timestamp;
    msg.reply(&ctx.http, format!("🏓 - {} ms", delay.num_milliseconds()))
        .await?;

    Ok(())
}

#[help]
#[individual_command_tip = "For specific information about a command use `help <command>`.\nParameters in () are optional whereas in <> are obligatory."]
#[max_levenshtein_distance(3)]
#[lacking_permissions = "Nothing"]
#[strikethrough_commands_tip_in_guild = ""]
#[strikethrough_commands_tip_in_dm = ""]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}
