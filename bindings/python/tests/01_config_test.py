import logging
from time import sleep
from random import randint
from links_bindings_python import SvcManual, CltManual
from links_connect.callbacks import LoggerCallback


log = logging.getLogger(__name__)

logger = LoggerCallback()


addr = f"127.0.0.1:{randint(2_000, 65_000)}"
max_connections = 1
io_timeout = 0.2


def test_config():
    import pytest

    with pytest.raises(Exception) as e_info:
        with CltManual(addr, logger) as clt:
            pass
    log.info(f"e_info: {e_info}")
    with pytest.raises(Exception) as e_info:
        with CltManual(addr, logger, **{"name": "clt"}) as clt:
            pass
    log.info(f"e_info: {e_info}")

    with SvcManual(addr, logger) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()

    sleep(0.1)  # yield

    with SvcManual(addr, logger, **{"name": "svc"}) as svc:
        log.info(f"svc: {svc}")
        assert not svc.is_connected()


if __name__ == "__main__":
    import pytest

    pytest.main([__file__])
