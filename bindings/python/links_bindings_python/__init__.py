from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum, auto

import logging

from .links_bindings_python import *

# https://www.maturin.rs/project_layout#pure-rust-project
__doc__ = links_bindings_python.__doc__
if hasattr(links_bindings_python, "__all__"):
    __all__ = links_bindings_python.__all__  # type: ignore


class ConType(Enum):
    Initiator = auto()
    Acceptor = auto()


@dataclass
class ConId(Enum):
    con_type: ConType
    name: str
    local: str
    peer: str


MsgDict = dict[str, str | int | float | bool | dict | list]  # Any?


class Callback(ABC):
    @abstractmethod
    def on_recv(self, con_id: ConId, msg: MsgDict) -> None:
        ...

    @abstractmethod
    def on_sent(self, con_id: ConId, msg: MsgDict) -> None:
        ...


class LoggerCallback(Callback):
    def __init__(self, sent_level=logging.INFO, recv_level=logging.INFO) -> None:
        super().__init__()
        self.sent_level = sent_level
        self.recv_level = recv_level

    def on_sent(self, con_id: ConId, msg: MsgDict):
        logging.getLogger(__name__).log(self.sent_level, f"on_sent: {con_id} {type(msg).__name__}({msg})")

    def on_recv(self, con_id: ConId, msg: MsgDict):
        logging.getLogger(__name__).log(self.recv_level, f"on_recv: {con_id} {type(msg).__name__}({msg})")

    def __str__(self) -> str:
        return f"{self.__class__.__name__}, sent_level={self.sent_level}, recv_level={self.recv_level}"
