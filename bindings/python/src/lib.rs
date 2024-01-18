use std::time::Duration;

use cfg_if::cfg_if;

pub mod callback;
pub mod prelude;
pub mod sender;

#[inline]
pub fn timeout_selector(priority_1: Option<f64>, priority_2: Option<f64>) -> Duration {
    match priority_1 {
        Some(timeout) => Duration::from_secs_f64(timeout),
        None => match priority_2 {
            Some(timeout) => Duration::from_secs_f64(timeout),
            None => Duration::from_secs(0),
        },
    }
}
cfg_if! {
    if #[cfg(feature = "unittest")]{

        use pyo3::prelude::*;
        use std::num::NonZeroUsize;

        #[pymodule]
        fn links_bindings_python(_py: Python, m: &PyModule) -> PyResult<()> {
                // IMPORTANT - py03 logger can cause background threads to block or deadlock as they need to acquire the GIL to log messages in python.
                // IMPORTANT - py03_log::init() will enable all logging including debug to be passed to python, even if PYTHON only logs INFO.
                // hence being conservative and only allowing WARN & above to be logged in release mode
                // https://docs.rs/pyo3-log/latest/pyo3_log/ LOGGING WILL DEAD LOCK PYTHON
                use links_nonblocking::{
                    prelude::{
                        unittest::setup::{self},
                        *},
                    unittest::setup::{
                        connection::{CltTest, CltTestSender, SvcTest, SvcTestSender},
                        protocol::{CltTestProtocolManual, SvcTestProtocolManual},
                    },
                };
                #[cfg(debug_assertions)]
                {
                    // pyo3_log::init();
                    let log = pyo3_log::try_init();
                    if log.is_err() {
                        log::info!("Looks like someone initialized logging prior to pyo3_log::try_init() -> {}", log.unwrap_err());
                    }
                }
                #[cfg(not(debug_assertions))]
                {
                    use pyo3_log::{Caching, Logger};
                    Logger::new(_py, Caching::LoggersAndLevels)?.filter(log::LevelFilter::Warn).install().expect("Someone installed a logger before us :-(");
                }

                create_callback_for_messenger!(CltTestProtocolManual, CltTestProtocolManualCallback);
                create_callback_for_messenger!(SvcTestProtocolManual, SvcTestProtocolManualCallback);
                create_clt_sender!(CltManual, CltTestSender, CltTestProtocolManual, CltTestProtocolManualCallback);
                create_svc_sender!(SvcManual, SvcTestSender, SvcTestProtocolManual, SvcTestProtocolManualCallback);
                #[pymethods]
                impl SvcManual {
                    #[new]
                    fn new(_py: Python<'_>, host: &str, callback: PyObject, max_connections: Option<NonZeroUsize>, io_timeout: Option<f64>, name: Option<&str>) -> Self {
                        let max_connections = max_connections.unwrap_or(NonZeroUsize::new(1).unwrap());
                        let callback = SvcTestProtocolManualCallback::new_ref(callback);
                        let protocol = SvcTestProtocolManual;
                        let sender = _py.allow_threads(move || SvcTest::bind(host, max_connections, callback, protocol, name).unwrap().into_sender_with_spawned_recver());
                        Self { sender, io_timeout }
                    }
                }
                #[pymethods]
                impl CltManual {
                    #[new]
                    fn new(_py: Python<'_>, host: &str, callback: PyObject, io_timeout: Option<f64>, name: Option<&str>) -> pyo3::PyResult<Self> {
                        let callback = CltTestProtocolManualCallback::new_ref(callback);
                        let protocol = CltTestProtocolManual;
                        match _py.allow_threads(move || CltTest::connect(host, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback, protocol, name)) {
                            Err(e) => Err(e.into()),
                            Ok(sender) => Ok(Self {
                                sender: _py.allow_threads(move || sender.into_sender_with_spawned_recver()),
                                io_timeout,
                            }),
                        }
                    }
                }
                m.add_class::<CltManual>()?;
                m.add_class::<SvcManual>()?;
                Ok(())
            }
    }
}
