import logging
from time import sleep
from random import randint
from links_connect import SvcManual, LoggerCallback

logging.basicConfig(format="%(levelname)s  %(asctime)-15s %(threadName)s %(name)s %(filename)s:%(lineno)d %(message)s")
logging.getLogger().setLevel(logging.DEBUG)
log = logging.getLogger(__name__)

callback = LoggerCallback()
addr = f"127.0.0.1:{randint(1_000, 65_000)}"

def test_svc():
    log.info("****** START ******")
    with SvcManual(addr, callback) as svc:
        log.info(f"svc: {svc}")
        sleep(0) # yield

    
    sleep(.5) # yield

    with SvcManual(addr, callback) as svc:
        log.info(f"svc: {svc}")
        sleep(0) # yield


if __name__ == "__main__":
    test_svc()