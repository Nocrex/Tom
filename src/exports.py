import json
import datetime
import aiofiles
from . import statics
from . import reports

async def simple_export(reps):
    steamids = []
    for reporter in reps._reporters.values():
        for report in reporter.reports:
            if not report.verified:
                continue
            steamids += [p for p, t in report.players.items() if t == reports.PlayerKind.CHEATER]
    steamids = set(map(lambda i: str(i), steamids))
    async with aiofiles.open(statics.ID_LIST_FILE, "w") as f:
        await f.write("\n".join(sorted(steamids)))

def steamid64_to_32(id: int) -> str:
    return f"[U:1:{id-statics.STEAMID64_OFFSET}]"

async def tfbd_export(reps):
    class PlayerRecord:
        def __init__(self) -> None:
            self.proof: list[str] = []
            self.attrs: set[str] = set()
            self.last_seen: int = 0
    
    steamids: dict[str, PlayerRecord] = {}
    for reporter in reps._reporters.values():
        for report in reporter.reports:
            if not report.verified:
                continue
            for steamid, kind in report.players.items():
                sid = steamid64_to_32(steamid)
                if sid not in steamids:
                    steamids[sid] = PlayerRecord()
                steamids[sid].proof.append(report.thread_url)
                steamids[sid].last_seen = max(steamids[sid].last_seen, int(report.timestamp.timestamp()))
                steamids[sid].attrs.add(kind)

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
            "attributes": list(s[1].attrs),
            "steamid": s[0],
            "proof": s[1].proof,
            "last_seen": {
                "time": s[1].last_seen
            }
        }, steamids.items()))
    }

    async with aiofiles.open(statics.TFBD_LIST_NAME, "w") as f:
        await f.write(json.dumps(contents, indent=4))

async def export(reports):
    await simple_export(reports)
    await tfbd_export(reports)
