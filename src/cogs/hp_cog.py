import discord
import typing
import logging
import re
import traceback
from io import StringIO
from discord.ext import commands, tasks
from discord import NotFound, app_commands, Object
from datetime import datetime, timezone, time
from typing import Iterator, AsyncIterator, TypeVar, AsyncGenerator

from .. import statics, steam
from ..reports import PlayerKind, Reports

logger = logging.getLogger(__name__)

class NotInThreadError(app_commands.AppCommandError): # custom error that is thrown when a thread-only command isn't called in a thread
    pass

def check_in_thread(interaction: discord.Interaction): # function to check if a command was called in a thread
    if not isinstance(interaction.channel, discord.Thread):
        raise NotInThreadError()
    return True

T = TypeVar("T")

async def join(it1: "Iterator[T]", it2: "AsyncIterator[T]")-> "AsyncGenerator[T]":
    while True:
        val = next(it1, "end")
        if val != "end":
            yield val
            continue
        val = await anext(it2, "end")
        if val != "end":
            yield val
            continue
        return

async def get_steamid(id: str) -> int | None: # resolve steam profile links and vanity urls
    if steam.STEAMID_REGEX.fullmatch(id) is not None:
        return int(id)
    
    m = re.match(steam.PERM_LINK_PATTERN, id)
    if m:
        return int(m.group(1))
    
    m = re.match(steam.VANITY_LINK_PATTERN, id)
    if m:
        return await steam.resolve_vanity_url(m.group(0))
    
    return None

def has_any_role(member: discord.Member, roles: typing.List[int]) -> bool: # check if a user has any of the roles passed
    if(any(role.id in roles for role in member.roles)):
        return True
    return False

class HPCog(commands.Cog):
    def __init__(self, bot: commands.Bot):
        self.bot: commands.Bot = bot
        self.toplist_needs_rebuild: bool = True
        self.toplist: typing.Optional[str] = None

    #### EVENT HANDLERS ####
    async def cog_load(self):
        self.reports: Reports = await Reports.load()
        logger.info("reports loaded")
        self.log_channel: discord.TextChannel = await self.bot.fetch_channel(statics.REPORT_CHANNEL_ID)
        self.error_channel: discord.TextChannel = await self.bot.fetch_channel(statics.ERROR_CHANNEL_ID)
        self.update_toplist.start() # start loop task to update the toplist regularly
        self.nag_officers.start()

    async def interaction_check(self, interaction: discord.Interaction) -> bool: 
        # usually used to check some condition for all app_commands in this Cog, but just log user and the command that was run
        options = []
        if "options" in interaction.data:
            for option in interaction.data["options"]:
                options.append(f"{option['name']}:'{option['value']}'")
        logger.info(f"{interaction.user.name} executed command: {interaction.command.name} {' '.join(options)}")
        return True

    async def cog_app_command_error(self, interaction: discord.Interaction, error: app_commands.AppCommandError):
        # error handler for all the checks in this cog
        if isinstance(error, app_commands.errors.MissingAnyRole):
            await interaction.response.send_message("You are not allowed to use this command", ephemeral=True)
        elif isinstance(error, NotInThreadError):
            await interaction.response.send_message("Cannot use this command outside of a thread", ephemeral=True)
        elif isinstance(error, app_commands.errors.CommandOnCooldown):
            await interaction.response.send_message("Issuing commands too quickly!", ephemeral=True)
        else:
            msg = "".join(traceback.format_exception(error))
            sio = StringIO(msg)
            await self.error_channel.send(file=discord.File(sio, filename="error.txt"))
            logger.error(msg)
    
    async def get_open_reports(self) -> list[discord.Thread]:
        open_reports: list[discord.Thread] = []
        tags_handled = [tag.id for tag in (statics.TAGS + [statics.CONFIRMED_TAG])] 
        channel = await self.bot.fetch_channel(statics.REPORT_FORUM.id)
        assert isinstance(channel, discord.ForumChannel)
        async for thread in join(iter(channel.threads), channel.archived_threads()):
            if thread.archived:
                continue
            tags = [t.id for t in thread.applied_tags]
            
            if statics.REPORT_TAG.id in tags and not any(map(lambda t: t in tags, tags_handled)):
                open_reports.append(thread)
        
        return open_reports
            
    @tasks.loop(time=time(hour=12))
    async def nag_officers(self):
        logger.info("Looking for reports to nag about")
        open_reports = await self.get_open_reports()
        
        now = datetime.now(timezone.utc)
        
        open_reports = [(t.jump_url, (now - t.created_at)) for t in open_reports if (now - t.created_at).days >= 1]
                
        if len(open_reports) > 0:
            open_reports.sort(key=lambda e: e[1], reverse=True)
            emb = discord.Embed(title="Stale reports", description="\n".join([f"{link} ({delta.days} days)" for link, delta in open_reports]))
            emb.colour = discord.Color.orange()
            await self.log_channel.send(embed=emb)
            logging.info(f"{len(open_reports)} stale reports")
        else:
            logging.info("Nothing to nag :)")
                

    @tasks.loop(minutes=1.0) # runs this function every minute
    async def update_toplist(self):
        # only run if necessary
        if not self.toplist_needs_rebuild:
            return
        logger.info("Toplist outdated, rebuilding")
        self.toplist_needs_rebuild = False

        top_reporters = self.reports.get_top_n(20)
        msg = ""
        for reporter in top_reporters:
            try:
                user = await self.bot.fetch_user(reporter.userid)
            except discord.NotFound: # Handle deleted user accounts
                class Mockuser:
                    def __init__(self, id):
                        self.global_name = id
                        self.mention = f"<@{id}>"
                user =  Mockuser(reporter.userid) # dummy class object, so the user id is used instead of the name

            msg += f"{reporter.points()}: {user.mention} ({user.global_name})\n"

        self.toplist = msg # Store toplist for later use
        logger.info("Toplist rebuilt")

        reported_count = self.reports.get_reported_count()
        await self.bot.change_presence(activity=discord.Activity(type=discord.ActivityType.watching, name=f"{reported_count} SteamIDs"))
        logger.info("Status updated")
        
    #### REGULAR USER COMMANDS ####
    @app_commands.command(
        name="points",
        description="Get a users (or your own) report point count"
    )
    @app_commands.describe(user="User to lookup points for, leave blank to get your own count")
    @app_commands.checks.cooldown(1, 5.0) # only allow one call every 5 seconds
    async def get_report_point_count(self, interaction: discord.Interaction, user: discord.Member | discord.User | None = None):
        if not user: # if no user was passed, look up points for the user calling the command
            user = interaction.user
        
        if not isinstance(user, discord.Member) or not isinstance(interaction.user, discord.Member):
            return

        reporter = self.reports.get(user.id)
        if not reporter: # user does not have a reporter entry
            await interaction.response.send_message("This user does not have any reports recorded", ephemeral=True)
            return

        embed = discord.Embed()
        embed.set_author(name=user.global_name, icon_url=user.display_avatar.url)
        embed.add_field(name="Report count", value=reporter.points(), inline=False)

        # detailed information that is only shown to officers
        if(has_any_role(interaction.user, statics.CONFIRM_ROLE_WHITELIST)):
            steamprofile = f"https://steamcommunity.com/profiles/{reporter.profile_id}" if reporter.profile_id else "not on record"
            embed.add_field(name="Steam profile", value=steamprofile, inline=False)
            
            recentreports = "\n".join(map(lambda r: r.message, reversed(reporter.reports[max(0,len(reporter.reports)-5):])))
            embed.add_field(name="Recent reports", value=recentreports, inline=False)

        await interaction.response.send_message(embed=embed, ephemeral=True) # ephemeral means that the message only shows up for the user using the command
        
    @app_commands.command(
        name="toplist",
        description="List the top 20 people based on report count"
    )
    @app_commands.checks.cooldown(1, 5.0)
    async def get_top_reporters(self, interaction: discord.Interaction):
        # sends back the toplist, if it was created already
        if not self.toplist:
            await interaction.response.send_message("Please wait, the toplist is being built", ephemeral=True)
            return
        await interaction.response.send_message(embed=discord.Embed(title="Top Reporters", description=self.toplist), ephemeral=True)

    @app_commands.command(
        name="lookup",
        description="Look up previous reports of a SteamID"
    )
    async def lookup(self, interaction: discord.Interaction, steamid: str):
        await interaction.response.defer(ephemeral=True)
        if not (steamid_i := await get_steamid(steamid.strip())):
            await interaction.followup.send("Invalid SteamID", ephemeral=True)
            return
        
        reports = self.reports.find_reported(steamid_i)
        lists = self.reports.check_external_lists(steamid_i)

        embed = discord.Embed(title=f"Information for {steamid_i}")

        if len(lists) > 0:
            embed.add_field(inline=False, name="External lists", value="\n".join(lists))
            embed.color = discord.Color.yellow()

        if len(reports) > 0:
            embed.insert_field_at(0, inline=False, name="Reports", value=
                '\n'.join(map(lambda r: f"{r[0]} [{r[2].kind.name}]" + (' -- (unverified)' if not r[1] else ''), reports)))
            embed.color = discord.Color.orange()
        
        if len(reports) + len(lists) == 0:
            embed.add_field(inline=False, name="No reports found", value="")
            embed.color = discord.Color.blue()

        await interaction.followup.send(embed=embed, ephemeral=True)

    #### OFFICER COMMANDS ####
    @app_commands.command(
        name="mark",
        description="Mark the current thread"
    )
    @app_commands.checks.has_any_role(*statics.CONFIRM_ROLE_WHITELIST)
    @app_commands.choices(tag=statics.TAG_CHOICES) # defines options for this parameter that show up in discord
    @app_commands.check(check_in_thread)
    async def mark(self, interaction: discord.Interaction, tag: app_commands.Choice[int]):
        # just adds the selected tag to the thread the command is called in
        if not isinstance((thread := interaction.channel), discord.Thread):
            await interaction.response.send_message("Can only be used in a report thread", ephemeral=True)
            return
            
        await thread.add_tags(statics.TAGS[tag.value])
        await interaction.response.send_message(f"Added tag {tag.name}", ephemeral=True)

    @app_commands.command(
        name="unmark",
        description="Unmark the current thread"
    )
    @app_commands.checks.has_any_role(*statics.CONFIRM_ROLE_WHITELIST)
    @app_commands.choices(tag=statics.TAG_CHOICES)
    @app_commands.check(check_in_thread)
    async def unmark(self, interaction: discord.Interaction, tag: app_commands.Choice[int]):
        # removes the selected tag from the thread this command was called in
        if not isinstance((thread := interaction.channel), discord.Thread):
            await interaction.response.send_message("Can only be used in a report thread", ephemeral=True)
            return
        await thread.remove_tags(statics.TAGS[tag.value])
        await interaction.response.send_message(f"Removed tag {tag.name}", ephemeral=True)
        
    @app_commands.command(
        name="approve",
        description="Approve the cheater report"
    )
    @app_commands.describe(
        points="Amount of points this report gives (default 1)", 
        steamids="Comma seperated list of reported steamids, ex. \"76561199796492647,76561199532619504\". Marks as Cheater by default, specify a different tag like \"76561199796492647[exploiter]\"",
        verified="If the steamids can be verified to be in the report",
        allow_duplicate="Skip checking if the user was already reported",
        reporter_steamid="SteamID of the person reporting, required if none has been logged yet",
        skip_reporter_steamid_check="Allow confirming even though the reporter has not provided a profile SteamID"
    )
    @app_commands.checks.has_any_role(*statics.CONFIRM_ROLE_WHITELIST)
    @app_commands.check(check_in_thread)
    @app_commands.rename(steamids="cheater_steamids")
    async def approve(self, 
        interaction: discord.Interaction, 
        steamids: str, 
        verified: bool,
        points: int | None = None, 
        reporter_steamid: typing.Optional[str] = None,
        allow_duplicate: bool = False,
        skip_reporter_steamid_check: bool = False
    ):            
        # Approves the report the command is executed in
        assert isinstance(interaction.channel, discord.Thread)
        thread: discord.Thread = interaction.channel
        owner = await self.bot.fetch_user(thread.owner_id)
        reporter = self.reports.get_or_create(thread.owner_id)
        
        if reporter.find_report(thread.jump_url): # look up if report was already approved
            await interaction.response.send_message("Report was already approved", ephemeral=True)
            return

        reporter_steamid_i = None
        if not reporter.profile_id and not reporter_steamid and not skip_reporter_steamid_check: # check if reporter has a steamid on record
            await interaction.response.send_message("Reporter does not have a steam profile ID associated", ephemeral=True)
            return
        elif reporter_steamid: # new steamid was passed to the command
            reporter_steamid_i = await get_steamid(reporter_steamid.strip())
            if not reporter_steamid_i:
                await interaction.response.send_message(f"Reporter SteamID \"{reporter_steamid}\" is not valid", ephemeral=True)
                return

        steamids_str = steamids.split(",") # get steamids from the command argument
        steamids_dict: dict[int, PlayerKind] = {}
        for steamid in steamids_str: # verify each steamid and convert to number
            type = PlayerKind.CHEATER
            if "[" in steamid and "]" in steamid:
                type_s = steamid[steamid.find("[")+1:steamid.find("]")]
                try:
                    type = PlayerKind(type_s)
                except ValueError:
                    await interaction.response.send_message(f"Unknown mark type {type_s}", ephemeral=True)
                    return
                steamid = steamid.replace(f"[{type.value}]", "")
            steamid_i = await get_steamid(steamid.strip())
            if not steamid_i:
                await interaction.response.send_message(f"Cheater SteamID \"{steamid}\" is not valid", ephemeral=True)
                return
            steamids_dict[steamid_i] = type
        
        if len(steamids_dict) == 0:
            await interaction.response.send_message("At least one cheater SteamID is required")
            return

        if points is None:
            points = len(steamids_dict)
        
        if not allow_duplicate:
            for steamid in steamids_dict: # check steamids if they were reported before
                reports = self.reports.find_reported(steamid)
                if len(reports) > 0 and (not verified or any(map(lambda r: r[1], reports))):
                    await interaction.response.send_message(f"Cheater {steamid} was already reported:\n{chr(10).join(map(lambda r: r[0] + (' -- (unverified)' if not r[1] else ''), reports))}", ephemeral=True)
                    return

        if reporter_steamid_i: # if a steamid for the reporter was passed, add it to the record and log it in the log channel
            reporter.profile_id = reporter_steamid_i
            await self.log_channel.send(f"Associated SteamID {reporter_steamid_i} with user {owner.mention}", silent=True)

        roles_added: list[Object] = []
        new_points = (prior_points := reporter.points()) + points
        
        try:
            thread_owner = interaction.guild.get_member(thread.owner_id) or await interaction.guild.fetch_member(thread.owner_id)
        except NotFound:
            thread_owner = None
            
        if thread_owner:
            for pts, role in statics.ROLE_MAPPING:
                if (prior_points < pts and
                    new_points >= pts and
                    not any(role.id == x.id for x in thread_owner.roles)):
                    roles_added.append(role)

        # log channel message
        msg = f"{thread.jump_url} {owner.mention} ({owner.display_name}) cheater exposed (+{points} points, {new_points} total)"
        # add report to internal record
        reporter.add_report(msg, points, steamids_dict, verified)

        try:
            if verified:
                await interaction.response.send_message(statics.Images.TOM_APPROVE) # send tom approved gif :D
            else:
                await interaction.response.send_message(f"-# unverified report[.]({statics.Images.TOM_APPROVE})")
        except NotFound:
            if verified:
                await interaction.channel.send(statics.Images.TOM_APPROVE) # send tom approved gif :D
            else:
                await interaction.channel.send(f"-# unverified report[.]({statics.Images.TOM_APPROVE})")
            
        # save data to json
        await self.reports.save()
        # mark toplist for rebuild
        self.toplist_needs_rebuild = True
        
        if roles_added:
            await thread_owner.add_roles(*roles_added)
            msg += f"\nAdded {', '.join(map(lambda x: f'<@&{x.id}>', roles_added))} role{'s' if len(roles_added) > 1 else ''} to user."

        # send confirmation message in log channel
        await self.log_channel.send(msg, silent=True, allowed_mentions=discord.AllowedMentions(roles=False))

        await thread.remove_tags(*statics.TAGS) # remove "Needs info", "Not a cheater" and "Already reported" tags
        await thread.add_tags(statics.CONFIRMED_TAG) # add "Confirmed" tag

    @app_commands.command(
        name="unapprove",
        description="Unapprove a report"
    )
    @app_commands.check(check_in_thread)
    @app_commands.checks.has_any_role(*statics.CONFIRM_ROLE_WHITELIST)
    async def unapprove(self, interaction: discord.Interaction):
        # removes a report for the current thread
        assert isinstance(interaction.channel, discord.Thread)
        thread = interaction.channel
            
        reporter = self.reports.get(thread.owner_id)
        if not reporter:
            await interaction.response.send_message("User does not have any reports", ephemeral=True)
            return
        
        prior_points = reporter.points()
        if not reporter.remove_report(thread.jump_url): # try to remove report from user, returns False if no matching reports were found
            await interaction.response.send_message("This thread has not been confirmed", ephemeral=True)
            return
        
        roles_removed: list[Object] = []
        new_points = reporter.points()
        
        try:
            thread_owner = interaction.guild.get_member(thread.owner_id) or await interaction.guild.fetch_member(thread.owner_id)
        except NotFound:
            thread_owner = None
            
        if thread_owner:
            for pts, role in statics.ROLE_MAPPING:
                if (prior_points >= pts and
                    new_points < pts and
                    any(role.id == x.id for x in thread_owner.roles)):
                    roles_removed.append(role)

        msg = f"{thread.jump_url} <@{thread.owner_id}> unapproved ({new_points} points)"
        if roles_removed:
            await thread_owner.remove_roles(*roles_removed)
            msg += f"\nRemoved {', '.join(map(lambda x: f'<@&{x.id}>', roles_removed))} role{'s' if len(roles_removed) > 1 else ''} from user."

        # save report record to disk
        await self.reports.save()
        # mark toplist for rebuild
        self.toplist_needs_rebuild = True
        # remove the "Confirmed" tag
        await thread.remove_tags(statics.CONFIRMED_TAG)
        # log unapproval in log channel
        await self.log_channel.send(msg, silent=True, allowed_mentions=discord.AllowedMentions(roles=False))
        # send a message that it was unapproved
        await interaction.response.send_message("Report unapproved")
        
    @app_commands.command(
        name="update_seen",
        description="Updates the time a cheater was last seen active"
    )
    @app_commands.describe(
        steamids="Comma seperated list of steamids, ex. \"76561199796492647,76561199532619504\".",
        date="Time at which the player was last seen (YYYY-MM-DD), leave empty to use current time",
    )
    @app_commands.checks.has_any_role(*statics.CONFIRM_ROLE_WHITELIST)
    @app_commands.check(check_in_thread)
    @app_commands.rename(steamids="cheater_steamids")
    async def update_seen(self, 
        interaction: discord.Interaction, 
        steamids: str, 
        date: str | None = None
    ):            
        assert isinstance(interaction.channel, discord.Thread)
        thread: discord.Thread = interaction.channel
        reporter = self.reports.get(thread.owner_id)
        
        if reporter is None or (report := reporter.find_report(thread.jump_url)) is None: # look up if report was already approved
            await interaction.response.send_message("Report was not approved", ephemeral=True)
            return

        steamids_str = steamids.split(",") # get steamids from the command argument
        steamids_list: list[int] = []
        for steamid in steamids_str: # verify each steamid and convert to number
            steamid_i = await get_steamid(steamid.strip())
            if not steamid_i:
                await interaction.response.send_message(f"Cheater SteamID \"{steamid}\" is not valid", ephemeral=True)
                return
            steamids_list.append(steamid_i)
        
        if len(steamids_list) == 0:
            await interaction.response.send_message("At least one cheater SteamID is required")
            return
            
        not_reported_ids = set(steamids_list) - set(report.players.keys())
        
        if len(not_reported_ids) > 0:
            await interaction.response.send_message(f"**These players were not in this report**\n{chr(10).join([str(id) for id in not_reported_ids])}", ephemeral=True)
            return
            
        if date is None:
            ts = datetime.now(timezone.utc)
        else:
            try:
                ts = datetime.fromisoformat(date)
            except ValueError:
                await interaction.response.send_message("Invalid date format", ephemeral=True)
                return
        
        msg = "**Updated last active time for**"
        
        for id in steamids_list:
            msg += f"\n`{id}`: {report.players[id].last_seen.date().isoformat()} -> {ts.date().isoformat()}"
            report.players[id].last_seen = ts
        
        await interaction.response.send_message(msg)
        
        # save data to json
        await self.reports.save()
        
    @app_commands.command(
        name="open_reports",
        description="Sends a list of open reports, can only be used in the modding channel"
    )
    @app_commands.checks.has_any_role(*statics.CONFIRM_ROLE_WHITELIST)
    @app_commands.checks.cooldown(1, 10*60, key=lambda int: int.guild_id)
    async def openreports(self, 
        interaction: discord.Interaction, 
    ):
        if interaction.channel_id != statics.REPORT_CHANNEL_ID:
            await interaction.response.send_message("Can only be used in the modding channel", ephemeral=True)
            return
            
        await interaction.response.defer()
            
        open_reports = await self.get_open_reports()
        
        if len(open_reports) > 0:
            open_reports.sort(key=lambda t: t.created_at)
            now = datetime.now(timezone.utc)
            emb = discord.Embed(title=f"{len(open_reports)} open reports", description="\n".join([f"{thread.jump_url} ({(now - thread.created_at).days} day(s) old)" for thread in open_reports]))
        else:
            emb = discord.Embed(title="No open reports")
            emb.color = discord.Color.green()
            
        await interaction.followup.send(embed=emb)

async def setup(bot: commands.Bot):
    # create new HPCog (just a self-contained module that provides commands) and add it to the bot
    # this function gets called by the bot.load_extension call in bot.py
    await bot.add_cog(HPCog(bot))
