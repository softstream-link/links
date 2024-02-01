import logging
from re import M
import pytest
from time import sleep
from random import randint
from links_bindings_python import SvcManual, CltManual

# , LoggerCallback
from links_connect.callbacks import LoggerCallback, DecoratorDriver, MemoryStoreCallback


log = logging.getLogger(__name__)
store = MemoryStoreCallback()
callback = DecoratorDriver() + store + LoggerCallback()
addr = f"127.0.0.1:{randint(1_000, 65_000)}"
max_connections = 1
io_timeout = 0.2


def test_svc():
    with SvcManual(
        addr,
        callback,
        max_connections,
        io_timeout,
    ) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()

    sleep(0.2)  # yield

    with SvcManual(addr, callback) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()

    sleep(0.2)  # yield

    with SvcManual(addr, callback) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()


def test_clt():
    with pytest.raises(Exception) as e_info:
        with CltManual(addr, callback) as clt:
            log.info(f"clt: {clt}")
    log.info(f"e_info: {e_info}")


def test_clt_svc():
    with (
        SvcManual(addr, callback, max_connections, io_timeout, "svc") as svc,
        CltManual(addr, callback, io_timeout, "clt") as clt,
    ):
        assert svc.is_connected()
        assert clt.is_connected()
        log.info(f"svc: {svc}")
        log.info(f"clt: {clt}")
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
    # pytest.main([__file__])
