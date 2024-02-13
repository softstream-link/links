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


def test_clt_2_svc_restart():

    for i in range(1, 11):
        log.info(f"\n{'*'*80} Start # {i} {'*'*80}")
        svc = SvcManual(addr, logger)
        # this call will busywait default io_timeout unless provided as argument, default should be visible in the log
        assert not svc.is_connected(io_timeout=0.1)
        clt = CltManual(addr, logger)

        log.info(f"svc: {svc}")
        log.info(f"clt: {clt}")
        assert svc.is_connected()
        assert clt.is_connected()
        clt.send({"Ping": {"ty": "P", "text": "ping"}})
        svc.send({"Pong": {"ty": "P", "text": "pong"}})

        svc.__exit__(None, None, None)
        clt.__exit__(None, None, None)

        assert not clt.is_connected()
        assert not svc.is_connected()  # this call shall not busywait after __exit__

        sleep(0.1)  # yield to release OSError: Address already in use (os error 48)


if __name__ == "__main__":
    import pytest

    pytest.main([__file__])
