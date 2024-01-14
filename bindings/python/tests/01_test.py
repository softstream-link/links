import logging
import pytest
from time import sleep
from random import randint
from links_connect import SvcManual, CltManual, LoggerCallback


logging.basicConfig(format="%(levelname)s  %(asctime)-15s %(threadName)s %(name)s %(filename)s:%(lineno)d %(message)s")
logging.getLogger().setLevel(logging.INFO)
log = logging.getLogger(__name__)

callback = LoggerCallback()
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


if __name__ == "__main__":
    test_svc()
    test_clt()
    test_clt_svc()
