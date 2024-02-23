import logging
from time import sleep
from random import randint
from links_bindings_python import SvcManual, CltManual
from links_connect.callbacks import LoggerCallback, DecoratorDriver, MemoryStoreCallback, on_recv, on_sent, ConId, Message


log = logging.getLogger(__name__)

logger = LoggerCallback()


addr = f"127.0.0.1:{randint(2_000, 65_000)}"
max_connections = 1
io_timeout = 0.2


def test_clt_not_connected_raises_exception():
    import pytest

    with pytest.raises(Exception) as e_info:
        with CltManual(addr, logger) as clt:
            log.info(f"clt: {clt}")
    log.info(f"e_info: {e_info}")


def test_clt_svc_connected_ping_pong():
    class NothingDecorator(DecoratorDriver):
        def __init__(self):
            super().__init__()

        @on_recv({})
        def on_recv_default(self, con_id: ConId, msg: Message):
            log.info(f"on_recv_default: {type(con_id)} {type(msg)} {con_id} {msg}")

        @on_sent({})
        def on_sent_default(self, con_id: ConId, msg: Message):
            log.info(f"on_sent_default: {type(con_id)} {type(msg)} {con_id} {msg}")

    store = MemoryStoreCallback()
    svc_decor = NothingDecorator()
    clt_decor = NothingDecorator()
    svc_decor = svc_decor + store + LoggerCallback()
    clt_decor = clt_decor + store + LoggerCallback()
    for i in range(1, 3):
        log.info(f"\n{'*'*80} Start # {i} {'*'*80}")
        store.clear()
        with (
            SvcManual(addr, svc_decor, **dict(name="svc")) as svc,
            CltManual(addr, clt_decor, **dict(name="clt")) as clt,
        ):
            assert svc.is_connected()
            assert clt.is_connected()

            log.info(f"svc: {svc}")
            log.info(f"clt: {clt}")
            assert clt_decor.sender == clt
            assert svc_decor.sender == svc
            clt.send({"Ping": {"ty": "P", "text": "ping"}})
            svc.send({"Pong": {"ty": "P", "text": "pong"}})

            found = store.find_recv(name=None, filter={"Ping": {}}, find_timeout=0.2)
            log.info(f"found: {found}")
            assert found is not None
            found = store.find_recv(name=None, filter={"Pong": {}}, find_timeout=0.2)
            log.info(f"found: {found}")
            assert found is not None

        sleep(0.1)  # Give time for the sockets to close


if __name__ == "__main__":
    import pytest

    pytest.main([__file__])
