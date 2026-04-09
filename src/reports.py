import asyncio
import enum
import json
import os
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Self

import aiofiles

from . import exports, statics, steam

# data classes to interact with the json data (I don't like working with dicts directly)
class PlayerKind(str, enum.Enum):
    CHEATER = "cheater"
    EXPLOITER = "exploiter"


# class that stores a single report,
# stores confirmation message, list of cheater steamids and points awarded for the report
class Report:
    def __init__(
        self,
        message: str,
        players: dict[int, PlayerKind],
        points: int,
        verified: bool,
        timestamp: datetime,
    ):
        self.message: str = message
        self.thread_url: str = message[: message.find(" ")]
        self.players: dict[int, PlayerKind] = players
        self.points: int = points
        self.verified: bool = verified
        self.timestamp: datetime = timestamp

    # creates a Report object from a json dict
    @staticmethod
    def from_json(json_report) -> Self:
        return Report(
            json_report["msg"],
            {int(p): PlayerKind(k) for p, k in json_report["players"].items()},
            json_report["points"],
            json_report["verified"],
            datetime.fromisoformat(json_report["date"]),
        )

    # creates a dict ready to be converted to json
    def to_json(self) -> Dict[str, Any]:
        return {
            "msg": self.message,
            "players": self.players,
            "points": self.points,
            "verified": self.verified,
            "date": self.timestamp.isoformat(),
        }


# class that represents a person reporting cheaters
# stores userid, list of Reports and steam profile id
class Reporter:
    def __init__(self, userid: int, reports: List[Report], profile_id: Optional[int]):
        self.userid: int = userid
        self.reports: List[Report] = reports
        self.profile_id: Optional[int] = profile_id

    # creates a new report for this reporter
    def add_report(
        self, msg: str, points: int, players: dict[int, PlayerKind], verified: bool
    ):
        self.reports.append(
            Report(msg, players, points, verified, datetime.now(timezone.utc))
        )

    # looks for a report with the passed thread link and removes it
    # returns True or False depending on if it succeeded or not
    def remove_report(self, link: str) -> bool:
        report = self.find_report(link)
        if report:
            self.reports.remove(report)
            return True
        return False

    # looks for a report based on a thread link
    def find_report(
        self, thread_link: str
    ) -> Optional[Report]:  # could return None or a Report
        for report in self.reports:
            if thread_link in report.message:
                return report
        return None

    @staticmethod
    def from_json(userid: int, json_map: dict) -> Self:
        reports: List[Report] = []
        for report in json_map["reports"]:
            reports.append(Report.from_json(report))
        return Reporter(userid, reports, json_map["profile_id"])

    # sums up all the points of the reports of this reporter
    def points(self) -> int:
        points = 0
        for report in self.reports:
            points += report.points
        return points

    # creates a dict ready to be converted to json
    def to_json(self):
        return {
            "reports": list(map(lambda r: r.to_json(), self.reports)),
            "count": self.points(),  # store total count aswell, is not used by the program but makes the json more usable on it's own
            "profile_id": self.profile_id,
        }


class Reports:
    def __init__(self, reporters: Dict[str, Reporter]):
        self._reporters: Dict[str, Reporter] = reporters
        self._lists: dict[str, set[int]] = dict()

    # gets the Reporter object for the given discord id, or creates a new empty one if it doesn't exist
    def get_or_create(self, reporter_idi: int) -> Reporter:
        reporter_id = str(reporter_idi)
        if reporter_id not in self._reporters:
            self._reporters[reporter_id] = Reporter(reporter_idi, [], None)
        return self._reporters[reporter_id]

    # gets the Reporter object for the given discord id, or None if it doesn't exist
    def get(self, reporter_idi: int) -> Optional[Reporter]:
        reporter_id = str(reporter_idi)
        if reporter_id in self._reporters:
            return self._reporters[reporter_id]

    # looks up if a cheater has been reported before, if yes returns the first Report
    def find_reported(self, steamid: int) -> List[tuple[str, bool, PlayerKind]]:
        matching_reports: List[tuple[str, bool, PlayerKind]] = []
        for reporter in self._reporters.values():
            for report in reporter.reports:
                if steamid in report.players:
                    matching_reports.append((report.thread_url, report.verified, report.players[steamid]))
        return matching_reports

    def check_external_lists(self, steamid: int) -> set[str]:
        return {l for l, i in self._lists.items() if steamid in i}

    # loads Report data from the data file
    @staticmethod
    async def load() -> Self:
        async with aiofiles.open(statics.REPORTS_DATA_FILE) as f:
            reports = json.loads(await f.read())
        r = Reports.from_json(reports)
        await r._load_lists(statics.EXTERNAL_LIST_DIR)
        return r

    # this sucks balls :steamhappy:
    async def _load_lists(self, list_dir):
        for filename in os.listdir(list_dir):
            path = os.path.join(list_dir, filename)
            async with aiofiles.open(path) as f:
                data = await f.read()

            list_name = ".".join(filename.split(".")[:-1])

            ids64 = set(
                map(
                    lambda match: int(match.group(0)),
                    steam.STEAMID_REGEX.finditer(data),
                )
            )
            ids3 = set(
                map(
                    lambda match: steam.sid3_to64(match.group(0)),
                    steam.STEAMID3_REGEX.finditer(data),
                )
            )

            self._lists[list_name] = ids64 | ids3

    # saves Report data to the data file
    async def save(self):
        async with aiofiles.open(statics.REPORTS_DATA_FILE, "w") as f:
            await f.write(json.dumps(self.to_json(), indent=4, sort_keys=True))
        await exports.export(self)

    # makes a list sorted by report count descending and returns the first n items
    def get_top_n(self, n) -> List[Reporter]:
        reporter_list = list(self._reporters.values())
        reporter_list.sort(key=lambda r: r.points(), reverse=True)
        return reporter_list[: min(20, len(reporter_list))]

    # creates a Reports object from a json dict
    @staticmethod
    def from_json(json_map: dict) -> Self:
        for key in json_map:
            json_map[key] = Reporter.from_json(int(key), json_map[key])
        return Reports(json_map)

    # creates a dict ready to be converted to json
    def to_json(self) -> Dict[str, dict]:
        reporters = {}
        for reporter in self._reporters:
            reporters[reporter] = self._reporters[reporter].to_json()
        return reporters

    def get_reported_count(self) -> int:
        return len(set(player for reporter in self._reporters.values() for report in reporter.reports for player in report.players))


async def test():
    reports = await Reports.load()
    await reports.save()


if __name__ == "__main__":
    # test code to mess about with the file format
    # only run when you execute this file directly
    asyncio.run(test())
