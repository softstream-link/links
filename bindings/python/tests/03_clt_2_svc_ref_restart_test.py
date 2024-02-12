import logging
from time import sleep
from random import randint
from links_bindings_python import SvcAuto, CltAuto
from links_connect.callbacks import LoggerCallback


log = logging.getLogger(__name__)

logger = LoggerCallback(sent_level=logging.DEBUG)


addr = f"127.0.0.1:{randint(2_000, 65_000)}"
max_connections = 1
io_timeout = 0.2


def test_clt_2_svc_ref_restart():

    for i in range(1, 3):
        log.info(f"\n{'*'*80} Start # {i} {'*'*80}")
        svc = SvcAuto(addr, logger)
        clt = CltAuto(addr, logger)

        log.info(f"svc: {svc}")
        log.info(f"clt: {clt}")
        assert svc.is_connected()
        assert clt.is_connected()
        clt.send({"Ping": {"ty": "P", "text": "ping"}})
        svc.send({"Pong": {"ty": "P", "text": "pong"}})

        svc.__exit__(None, None, None)
        clt.__exit__(None, None, None)

        # tests special case of is_connected() after __exit__ which should be False
        # particularly relevant to Svc as its implementation is going to probe an rx_sender channel
        # to see if Poller generated a new connection for the pool but it is in the channel and not yet added
        # to the pool
        assert not svc.is_connected()
        assert not clt.is_connected()

        sleep(0.1)  # yield to release OSError: Address already in use (os error 48)


if __name__ == "__main__":
    import pytest

    pytest.main([__file__])
