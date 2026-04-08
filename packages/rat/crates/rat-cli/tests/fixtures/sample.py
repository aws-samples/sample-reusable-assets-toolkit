from dataclasses import dataclass
from typing import Optional


@dataclass
class Config:
    name: str
    debug: bool = False


class Processor:
    def __init__(self, config: Config):
        self.config = config

    def run(self, data: list[str]) -> list[str]:
        return [item.upper() for item in data]


def create_processor(name: str) -> Processor:
    config = Config(name=name)
    return Processor(config)


MAX_WORKERS = 4
