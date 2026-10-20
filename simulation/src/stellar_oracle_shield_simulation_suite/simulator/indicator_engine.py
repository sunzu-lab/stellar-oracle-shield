from .config import ScorerConfig

import socket
import json

from dataclasses import asdict, dataclass
from datetime import datetime, timezone


@dataclass
class MidPrice:
    ts: datetime
    venue: str
    instrument: str
    mid_price: float

    def to_json(self) -> str:
        data = asdict(self)
        data["ts"] = (
            self.ts.astimezone(timezone.utc)
            .replace(tzinfo=None)
            .isoformat(sep=" ", timespec="milliseconds")
        )
        return json.dumps(data)


class IndicatorEngine:

    def __init__(self, config: ScorerConfig):
        self.config = config

    def set_midprice(self, venue: str, instrument: str, mid_price: float):
        message = MidPrice(
            ts=datetime.now(), venue=venue, instrument=instrument, mid_price=mid_price
        )
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
            sock.sendto(
                message.to_json().encode("utf-8"),
                (self.config.host, self.config.port),
            )
