import json, datetime, aiofiles
from . import statics
from . import reports

async def simple_export(reports):
    steamids = []
    for reporter in reports._reporters.values():
        for report in reporter.reports:
            if not report.verified:
                continue
            steamids += report.steamids
    steamids = set(map(lambda i: str(i), steamids))
    async with aiofiles.open(statics.ID_LIST_FILE, "w") as f:
        await f.write("\n".join(sorted(steamids)))

def steamid64_to_32(id: int) -> str:
    return f"[U:1:{id-statics.STEAMID64_OFFSET}]"

async def tfbd_export(reports):
    steamids = {}
    for reporter in reports._reporters.values():
        for report in reporter.reports:
            if not report.verified:
                continue
            for sid in report.steamids:
                sid = steamid64_to_32(sid)
                if sid not in steamids:
                    steamids[sid] = [[],0]
                steamids[sid][0] += [report.thread_url]
                steamids[sid][1] = max(steamids[sid][1], int(report.timestamp.timestamp()))

    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M:%S")
    contents = {
         "$schema": "https://raw.githubusercontent.com/PazerOP/tf2_bot_detector/master/schemas/v3/playerlist.schema.json",
        "file_info": {
            "authors": [ "All contributors in the hackerpolice channel" ],
            "description": f"List of cheaters reported in the hackerpolice channel on the Vorobey discord server, last updated {now}",
            "title": f"vorobey-hackerpolice - {now}",
            "update_url": f"https://raw.githubusercontent.com/Nocrex/Tom/refs/heads/main/{statics.TFBD_LIST_NAME}"
        },
        "players": list(map(lambda s: {
            "attributes": ["cheater"],
            "steamid": s[0],
            "proof": s[1][0],
            "last_seen": {
                "time": s[1][1]
            }
        }, steamids.items()))
    }

    async with aiofiles.open(statics.TFBD_LIST_NAME, "w") as f:
        await f.write(json.dumps(contents, indent=4))

async def export(reports):
    await simple_export(reports)
    await tfbd_export(reports)
