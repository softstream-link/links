import logging
from re import M
import pytest
from time import sleep
from random import randint
from links_bindings_python import SvcManual, CltManual

# , LoggerCallback
from links_connect.callbacks import LoggerCallback, DecoratorDriver, MemoryStoreCallback


log = logging.getLogger(__name__)

logger = LoggerCallback()


addr = f"127.0.0.1:{randint(2_000, 65_000)}"
max_connections = 1
io_timeout = 0.2


def test_svc_port_reuse():
    with SvcManual(addr, logger) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()

    sleep(0.1)  # yield

    with SvcManual(addr, logger) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()

    sleep(0.1)  # yield

    with SvcManual(addr, logger) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()


def test_clt_not_connected_raises_exception():
    with pytest.raises(Exception) as e_info:
        with CltManual(addr, logger) as clt:
            log.info(f"clt: {clt}")
    log.info(f"e_info: {e_info}")


def test_clt_svc_connected_ping_pong():
    store = MemoryStoreCallback()
    svc_decor = DecoratorDriver()
    clt_decor = DecoratorDriver()
    svc_decor = svc_decor + store + LoggerCallback()
    clt_decor = clt_decor + store + LoggerCallback()
    with (
        SvcManual(addr, svc_decor, max_connections, io_timeout, "svc") as svc,
        CltManual(addr, clt_decor, io_timeout, "clt") as clt,
    ):
        assert svc.is_connected()
        assert clt.is_connected()

        log.info(f"svc: {svc}")
        log.info(f"clt: {clt}")
        assert clt_decor.sender == clt
        assert svc_decor.sender == svc
        clt.send({"Ping": {"ty": "P", "text": "ping"}})
        svc.send({"Pong": {"ty": "P", "text": "pong"}})

        found = store.find_recv(name=None, filter={"Ping": {}}, io_timeout=0.2)
        log.info(f"found: {found}")
        assert found is not None
        found = store.find_recv(name=None, filter={"Pong": {}}, io_timeout=0.2)
        log.info(f"found: {found}")
        assert found is not None


if __name__ == "__main__":
    pytest.main([__file__])
