use std::time::Duration;

use cfg_if::cfg_if;

pub mod atexit;
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
        use log::info;
        use pyo3::types::PyDict;
        use serde::Serialize;
        create_register_atexit!();

        #[pymodule]
        fn links_bindings_python(_py: Python, m: &PyModule) -> PyResult<()> {
                register_atexit()?;

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
                create_clt_sender!(CltManual, CltTestSender, CltTestProtocolManual, CltTestProtocolManualCallback, "unittest");
                create_svc_sender!(SvcManual, SvcTestSender, SvcTestProtocolManual, SvcTestProtocolManualCallback, "unittest");

                #[derive(Serialize)]
                struct SvcManualConfig {
                    pub max_connections: NonZeroUsize,
                    pub io_timeout: Option<f64>,
                    pub name: Option<String>,
                }
                impl Default for SvcManualConfig {
                    fn default() -> Self {
                        Self { max_connections: NonZeroUsize::new(1).unwrap(), io_timeout: Some(0.5), name: Some(asserted_short_name!("SvcManual", SvcManual).to_owned()) }
                    }
                }
                impl From<&PyDict> for SvcManualConfig {
                    fn from(kwargs: &PyDict) -> Self {
                        let default = Self::default();
                        let max_connections = kwargs.get_item("max_connections").unwrap().map_or(default.max_connections, |any| NonZeroUsize::new(any.extract::<usize>().unwrap()).unwrap());
                        let io_timeout = kwargs.get_item("io_timeout").unwrap().map_or(default.io_timeout, |any| Some(any.extract::<f64>().unwrap()));
                        let name = kwargs.get_item("name").unwrap().map_or(default.name, |any| Some(any.extract::<String>().unwrap()));
                        Self { max_connections, io_timeout, name }
                    }
                }
                #[pymethods]
                impl SvcManual {
                    #[new]
                    #[pyo3(signature = (host, callback, **kwargs ))]
                    fn new(_py: Python<'_>, host: &str, callback: PyObject, kwargs: Option<&PyDict>) -> PyResult<Py<Self>> {
                        let config = kwargs.map_or(SvcManualConfig::default(), SvcManualConfig::from);
                        info!("{}: effective config: {} with kwargs: {:?}", asserted_short_name!("SvcManual", Self), serde_json::to_string(&config).unwrap(), kwargs);
                        let sender = {
                            let callback = SvcTestProtocolManualCallback::new_ref(callback.clone());
                            let protocol = SvcTestProtocolManual;
                            let sender = _py.allow_threads(move || SvcTest::bind(host, config.max_connections, callback, protocol, config.name.as_deref()))?.into_sender_with_spawned_recver();
                            Py::new(_py,Self { sender, io_timeout: config.io_timeout })?
                        };
                        patch_callback_if_settable_sender!(_py, sender, callback, asserted_short_name!("SvcManual", Self));
                        Ok(sender)
                    }
                }
                #[derive(Serialize)]
                struct CltManualConfig {
                    pub connect_timeout: f64,
                    pub retry_connect_after: f64,
                    pub io_timeout: Option<f64>,
                    pub name: Option<String>,
                }
                impl Default for CltManualConfig {
                    fn default() -> Self {
                        Self { connect_timeout: 1., retry_connect_after: 0.1, io_timeout: Some(0.5), name: Some(asserted_short_name!("CltManual", CltManual).to_owned()) }
                    }
                }
                impl From<&PyDict> for CltManualConfig {
                    fn from(kwargs: &PyDict) -> Self {
                        let default = Self::default();
                        let connect_timeout = kwargs.get_item("connect_timeout").unwrap().map_or(default.connect_timeout, |any| any.extract::<f64>().unwrap());
                        let retry_connect_after = kwargs.get_item("retry_connect_after").unwrap().map_or(default.retry_connect_after, |any| any.extract::<f64>().unwrap());
                        let io_timeout = kwargs.get_item("io_timeout").unwrap().map_or(default.io_timeout, |any| Some(any.extract::<f64>().unwrap()));
                        let name = kwargs.get_item("name").unwrap().map_or(default.name, |any| Some(any.extract::<String>().unwrap()));
                        Self { connect_timeout, retry_connect_after, io_timeout, name }

                    }
                }
                #[pymethods]
                impl CltManual {
                    #[new]
                    #[pyo3(signature = (host, callback, **kwargs ))]
                    fn new(_py: Python<'_>, host: &str, callback: PyObject, kwargs: Option<&PyDict>) -> PyResult<Py<Self>> {
                        let config = kwargs.map_or(CltManualConfig::default(), CltManualConfig::from);
                        info!("{}: effective config: {} with kwargs: {:?}", asserted_short_name!("CltManual", Self), serde_json::to_string(&config).unwrap(), kwargs) ;
                        let sender = {
                            let callback = CltTestProtocolManualCallback::new_ref(callback.clone());
                            let protocol = CltTestProtocolManual;
                            let sender = _py.allow_threads(move || CltTest::connect(host, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback, protocol, config.name.as_deref()))?;
                            Py::new(_py, Self { sender: sender.into_sender_with_spawned_recver(), io_timeout: config.io_timeout })?
                        };
                        patch_callback_if_settable_sender!(_py, sender, callback, asserted_short_name!("CltManual", Self));
                        Ok(sender)
                    }
                }
                m.add_class::<CltManual>()?;
                m.add_class::<SvcManual>()?;
                Ok(())
            }
    }
}
