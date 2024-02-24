import logging
from time import sleep
from random import randint
from links_bindings_python import SvcManual, CltManual
from links_connect.callbacks import LoggerCallback, DecoratorDriver, MemoryStoreCallback, on_recv, on_sent, ConId, Message


log = logging.getLogger(__name__)

logger = LoggerCallback()


addr = f"127.0.0.1:{randint(2_000, 65_000)}"
max_connections = 1
io_timeout = 0.1


def test_clt_not_connected_raises_exception():
    import pytest

    with pytest.raises(Exception) as e_info:
        with CltManual(addr, logger) as clt:
            log.info(f"clt: {clt}")
    log.info(f"e_info: {e_info}")


def test_clt_svc_connected_ping_pong():
    class PongDecorator(DecoratorDriver):
        def __init__(self):
            super().__init__()

        @on_recv({})
        def on_recv_default(self, con_id: ConId, msg: Message):
            log.info(f"{self.__class__.__name__}.on_recv_default: {con_id} {type(msg).__name__}({msg})")
            self.sender.send({"Pong": {"ty": "P", "text": "pong"}})

        @on_sent({})
        def on_sent_default(self, con_id: ConId, msg: Message):
            log.info(f"{self.__class__.__name__}.on_sent_default: {con_id} {type(msg).__name__}({msg})")

    store = MemoryStoreCallback()
    svc_clbk = PongDecorator() + store
    clt_clbk = LoggerCallback() + store
    for i in range(1, 11):
        log.info(f"\n{'*'*80} Start # {i} {'*'*80}")
        store.clear()
        with (
            SvcManual(addr, svc_clbk, **dict(name="svc")) as svc,
            CltManual(addr, clt_clbk, **dict(name="clt")) as clt,
        ):
            assert svc.is_connected()
            assert clt.is_connected()

            log.info(f"svc: {svc}")
            log.info(f"clt: {clt}")

            assert svc_clbk.sender == svc
            # this will send the message and and almost immediately drop clt & svc, this in turn should
            # fails PongDecorator.on_recv_default because sender would not be invalid, however it should not
            # panic but only raise an exception which should visible in the log as ERROR
            clt.send({"Ping": {"ty": "P", "text": "ping"}})

        sleep(0.1)  # OSError: Address already in use (os error 48)


if __name__ == "__main__":
    import pytest

    pytest.main([__file__])
