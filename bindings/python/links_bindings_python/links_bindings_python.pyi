# this is a cog generated python IDE interface file

# [[[cog
# import cog
# names = ["CltManual", "SvcManual"]
# cog.out(
# f"""
# from types import TracebackType
# from links_connect.callbacks import Callback
# """
# )
# for name in names:
#   cog.out(
# f"""
# class {name}:
#     def __init__(self, host: str, callback: Callback, **kwargs) -> None: ...
#     def __enter__(self) -> {name}: ...
#     def __exit__(self, exc_type: type[BaseException] | None, exc_value: BaseException | None, traceback: TracebackType | None) -> None: ...
#     def send(self, msg: dict, io_timeout: float | None = None): ...
#     def is_connected(self, io_timeout: float | None = None) -> bool: ...
#     @property
#     def msg_samples(self) -> list[str]: ...
#
# """)
#
# ]]]

from types import TracebackType
from links_connect.callbacks import Callback

class CltManual:
    def __init__(self, host: str, callback: Callback, **kwargs) -> None: ...
    def __enter__(self) -> CltManual: ...
    def __exit__(self, exc_type: type[BaseException] | None, exc_value: BaseException | None, traceback: TracebackType | None) -> None: ...
    def send(self, msg: dict, io_timeout: float | None = None): ...
    def is_connected(self, io_timeout: float | None = None) -> bool: ...
    @property
    def msg_samples(self) -> list[str]: ...


class SvcManual:
    def __init__(self, host: str, callback: Callback, **kwargs) -> None: ...
    def __enter__(self) -> SvcManual: ...
    def __exit__(self, exc_type: type[BaseException] | None, exc_value: BaseException | None, traceback: TracebackType | None) -> None: ...
    def send(self, msg: dict, io_timeout: float | None = None): ...
    def is_connected(self, io_timeout: float | None = None) -> bool: ...
    @property
    def msg_samples(self) -> list[str]: ...

# [[[end]]]
