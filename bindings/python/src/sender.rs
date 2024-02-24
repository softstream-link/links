/// Macro for generating a [macro@pyo3::pyclass] extension over a [links_nonblocking::prelude::CltSender]
///
/// # Arguments
/// * `name` - Name python extension class to be created
/// * `sender` - Type of the [links_nonblocking::prelude::CltSender] type to be extended
/// * `protocol` - Type of the [links_nonblocking::prelude::Protocol] type to be used by the sender
/// * `callback` - Type of the [links_nonblocking::prelude::CallbackRecvSend] type to be used by the sender
/// * `module` - Name of the python module that will contains the sender
///
/// # Result
/// ## Methods & signatures
/// * `send` - (msg: dict, io_timeout: float | None = None) -> None
/// * `is_connected` - (io_timeout: float | None = None) -> bool
/// ## Python Dunders
/// * `__enter__` - context manager
/// * `__exit__` - context manager
/// * `__del__` - destructor
/// * `__repr__` - representation
///
/// # Important
/// * all arguments must be fully qualified types and cannot have unresolved generics
/// * The resulting struct is missing a constructor, you need to implement it yourself, see example below. This is due
/// to the fact that constructor needs to provide necessary arguments to the `Protocol` which are not known to this macro
/// ```ignore
/// create_clt_sender!(CltManual, CltTestSender, CltTestProtocolManual, CltTestProtocolManualCallback);
/// #[pymethods]
/// impl CltManual {
///     #[new]
///     fn new(_py: Python<'_>, host: &str, callback: PyObject) -> Self {
///         let protocol = CltTestProtocolManual::default();
///         let callback = CltTestProtocolManualCallback::new_ref(callback);
///         let clt = CltTest::connect(host, default_connect_timeout(), default_connect_retry_after(), callback, protocol, Some("py-clt"))
///             .unwrap()
///             .into_sender_with_spawned_recver();
///         Self { sender: clt, io_timeout: None }
///     }
/// }
/// ```
#[macro_export]
macro_rules! create_clt_sender(
    ($name:ident, $sender:ident, $protocol:ident, $callback:ident, $module:literal) => {
        $crate::create_struct!($name, $sender, $protocol, $callback, $module);
        $crate::send!($name, $sender, $protocol);
        $crate::clt_is_connected!($name, $sender, $protocol);
        $crate::clt__repr__!($name);
        $crate::__enter__!($name);
        $crate::__exit__!($name);
        $crate::__del__!($name);
    }
);

/// Macro for generating a [macro@pyo3::pyclass] extension over a [links_nonblocking::prelude::SvcSender]
///
/// # Arguments
/// * `name` - Name python extension class to be created
/// * `sender` - Type of the [links_nonblocking::prelude::SvcSender] type to be extended
/// * `protocol` - Type of the [links_nonblocking::prelude::Protocol] type to be used by the sender
/// * `callback` - Type of the [links_nonblocking::prelude::CallbackRecvSend] type to be used by the sender
/// * `module` - Name of the python module that will contains the sender
///
/// # Result
/// ## Methods & signatures
/// * `send` - (msg: dict, io_timeout: float | None = None) -> None
/// * `is_connected` - (io_timeout: float | None = None) -> bool
/// ## Python Dunders
/// * `__enter__` - context manager
/// * `__exit__` - context manager
/// * `__del__` - destructor
/// * `__repr__` - representation
///
/// # Important
/// * all arguments must be fully qualified types and cannot have unresolved generics
/// * The resulting struct is missing a constructor, you need to implement it yourself, see example below. This is due
/// to the fact that constructor needs to provide necessary arguments to the `Protocol` which are not known to this macro
#[macro_export]
macro_rules! create_svc_sender(
    ($name:ident, $sender:ident, $protocol:ident, $callback:ident, $module:literal) => {
        $crate::create_struct!($name, $sender, $protocol, $callback, $module);
        $crate::send!($name, $sender, $protocol);
        $crate::svc_is_connected!($name, $sender, $protocol);
        $crate::svc__repr__!($name);
        $crate::__enter__!($name);
        $crate::__exit__!($name);
        $crate::__del__!($name);
    }
);

/// Marco will patch the `PyObject` if it is an instance of `links_connect.callbacks.SettableSender`
/// by setting the `sender` attribute to the provided `sender` argument.
///
/// # Arguments
/// * `py` - python interpreter token [`Python<'_>`]
/// * `sender` - This is a Rust class instance of type [macro@pyo3::pyclass]
/// * `callback` - This is a [pyo3::PyObject] representing a callback for `on_sent` and `on_recv` events for which a reference will be stored in the `sender` property of the
/// * `pyclass_name` - This is a string literal representing the name of the python class for which the callback is being patched, it is used for logging purposes
#[macro_export]
macro_rules! patch_callback_if_settable_sender {
    ($py:ident, $sender:ident, $callback:ident, $pyclass_name:expr) => {{
        let locals = pyo3::types::PyDict::new($py);
        locals.set_item("callback", &$callback)?;
        match pyo3::Python::run($py, "from links_connect.callbacks import SettableSender; is_settable_sender = isinstance(callback, SettableSender)", None, Some(locals)) {
            Ok(_) => {
                let is_settable_sender: bool = locals.get_item("is_settable_sender").map_or(false, |opt| opt.map_or(false, |any| any.extract::<bool>().map_or(false, |v| v)));
                if is_settable_sender {
                    log::info!(
                        "{}: callback is an instance of `links_connect.callbacks.SettableSender`, setting callback.sender: {}",
                        $pyclass_name,
                        $sender.borrow($py).__repr__($py)
                    );
                    $callback.setattr($py, "sender", $sender.clone_ref($py))?; // faster then $sender.clone() when gil is held https://docs.rs/pyo3/latest/pyo3/struct.Py.html#method.clone_ref
                } else {
                    log::info!("{}: callback is NOT an instance of `links_connect.callbacks.SettableSender`, callback.sender will NOT be set", $pyclass_name);
                }
            }
            Err(err) => {
                log::warn!("failed to validate if callback is an instance of `links_connect.callbacks.SettableSender` err: {:?}", err);
            }
        }
    }};
}

#[macro_export]
macro_rules! create_struct(
    ($name:ident, $sender:ident, $protocol:ident, $callback:ident, $py_module:literal) => {
        #[doc = concat!("[`", stringify!($name), "`] is a python extension module for [`", stringify!($sender), "`] sender, implementing [`", stringify!($protocol) ,"`] protocol", )]
        #[pyo3::pyclass(frozen, module = $py_module)]
        pub struct $name {
            sender: spin::Mutex<Option<$sender<$protocol, $callback>>>,
            con_id: links_nonblocking::prelude::ConId,
            io_timeout: Option<f64>,
        }
    }
);

#[macro_export]
macro_rules! send(
    ($name:ident, $sender:ident, $protocol:ident ) => {
        #[pyo3::pymethods]
        impl $name{
            #[doc = concat!(
                "[`", stringify!($name), ".send`] converts `msg` argument into [`", stringify!($protocol) ,"`] protocol format and sends it to connected peer, will raise exception if `io_timeout` is reached.",
                "\n[`", stringify!($name), ".msg_samples`] provides valid sample messages for [`", stringify!($protocol) ,"`] protocol."
            )]
            fn send(&self, _py: pyo3::Python<'_>, msg: pyo3::Py<pyo3::types::PyDict>, io_timeout: Option<f64>) -> pyo3::PyResult<()> {
                let io_timeout = $crate::timeout_selector(io_timeout, self.io_timeout);
                let json_module = pyo3::types::PyModule::import(_py, "json")?;
                let json: String = json_module.getattr("dumps")?.call1((msg,))?.extract()?;
                let mut msg = serde_json::from_str(json.as_str()).unwrap();

                _py.allow_threads(move || match &mut *self.sender.lock() {
                    Some(sender) => match sender.send_busywait_timeout(&mut msg, io_timeout)? {
                        links_nonblocking::prelude::SendStatus::Completed => Ok(()),
                        links_nonblocking::prelude::SendStatus::WouldBlock => Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, format!("Message not delivered due timeout: {:?}, msg: {}", io_timeout, json)).into()),
                    },
                    None => Err(pyo3::exceptions::PyConnectionError::new_err(format!("{}({}) calling '{}.send' after '{}.__exit__(), Did you create a links_connect.callbacks.DecoratorDriver which is trying to send reply after Sender connection was closed?'", stringify!($name), self.con_id, stringify!($name), stringify!($name) ))),
                })
            }
        }
    }
);

#[macro_export]
macro_rules! clt_is_connected(
    ($name:ident, $sender:ident, $protocol:ident) => {
        #[pyo3::pymethods]
        impl $name{
            #[doc = concat!(
                "[`", stringify!($name), ".is_connected`] returns status of connection for [`", stringify!($sender) , "`] according to the [`", stringify!($protocol) ,"`] protocol implementation. ",
                "Note if `io_timeout` is not provided a default will be used if default is not set it is assumed ZERO"
            )]
            fn is_connected(&self, _py: pyo3::Python<'_>, io_timeout: Option<f64>) -> bool {
                // No reason for use default timeout for clt as it won't establish a connection no matter how long it will wait
                let io_timeout = $crate::timeout_selector(io_timeout, None);
                _py.allow_threads(move || match & *self.sender.lock() {
                    Some(sender) => sender.is_connected_busywait_timeout(io_timeout),
                    None => false,
                })
            }
        }
    }
);
#[macro_export]
macro_rules! svc_is_connected(
    ($name:ident, $sender:ident, $protocol:ident) => {
        #[pyo3::pymethods]
        impl $name{
            #[doc = concat!(
                "[`", stringify!($name), ".is_connected`] returns status of the next connection in the pool of [`", stringify!($sender) , "`] according to the [`", stringify!($protocol) ,"`] protocol implementation. ",
                "Note if `io_timeout` is not provided a default will be used if default is not set it is assumed ZERO"
            )]
            fn is_connected(&self, _py: Python<'_>, io_timeout: Option<f64>) -> bool {
                let io_timeout = $crate::timeout_selector(io_timeout, self.io_timeout);
                _py.allow_threads(move || match &mut *self.sender.lock() {
                    Some(sender) => sender.is_next_connected_busywait_timeout(io_timeout),
                    None => false,
                })

            }
        }
    }
);

#[macro_export]
macro_rules! clt__repr__(
    ($name:ident) => {
        #[pyo3::pymethods]
        impl $name{
            fn __repr__(&self, _py: pyo3::Python<'_>) -> String {
                let is_connected= self.is_connected(_py, None);
                format!("{}({}, is_connected: {})", stringify!($name), self.con_id, is_connected)
            }
        }
    }
);
#[macro_export]
macro_rules! svc__repr__(
    ($name:ident) => {
        #[pyo3::pymethods]
        impl $name{
            fn __repr__(&self, _py: pyo3::Python<'_>) -> String {
                _py.allow_threads(move || { match &mut *self.sender.lock() {
                        Some(sender) => {
                            let is_connected = sender.is_next_connected();
                            let status = if !is_connected {
                                    format!("{}({}, is_connected: {})", stringify!($name), self.con_id, false)
                                } else {
                                    let num = sender.len();
                                    let max = sender.max_connections().get() / links_nonblocking::prelude::SVC_MAX_CONNECTIONS_2_POOL_SIZE_FACTOR.get();
                                    let connections = sender.iter().map(|(_, s)| format!("[{}, is_connected: {}]", s.con_id(), s.is_connected())).collect::<Vec<_>>().join(",");
                                    format!("{}({} of {} {})", stringify!($name), num, max, connections)
                                };
                            format!("{}", status)
                        }
                        None => format!("{}({}, is_connected: {})", stringify!($name), self.con_id, false),
                    }
                })
            }
        }
    }
);

#[macro_export]
macro_rules! __enter__(
    ($name:ident) => {
        /// Returns self.
        #[pyo3::pymethods]
        impl $name{
            fn __enter__(slf: pyo3::Py<Self>) -> pyo3::Py<Self> {
                slf
            }
        }
    }
);
#[macro_export]
macro_rules! __exit__(
    ($name:ident) => {
        /// Terminates connection by setting [Option] self.sender to [None], which will intern drop inner value of the option.
        #[pyo3::pymethods]
        impl $name{
            fn __exit__(&self, _py: pyo3::Python<'_>, _exc_type: Option<&pyo3::PyAny>, _exc_value: Option<&pyo3::PyAny>, _traceback: Option<&pyo3::PyAny>) {
                _py.allow_threads(move || {
                    let opt = &mut *self.sender.lock();
                    if opt.is_some() {
                        let sender = opt.take();
                        drop(sender)
                    }
                })
            }
        }
    }
);
#[macro_export]
macro_rules! __del__(
    ($name:ident) => {
        /// Terminates connection by setting [Option] self.sender to [None], which will intern drop inner value of the option.
        #[pyo3::pymethods]
        impl $name{
            fn __del__(&self, _py: pyo3::Python<'_>) {
                _py.allow_threads(move || {
                    let opt = &mut *self.sender.lock();
                    if opt.is_some() {
                        let sender = opt.take();
                        drop(sender)
                    }
                })
            }
        }
    }
);

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use crate::{callback, prelude::*};
    use callback::ConId;
    use links_nonblocking::{
        prelude::{
            unittest::setup::{
                self,
                net::{default_connect_retry_after, default_connect_timeout},
            },
            *,
        },
        unittest::setup::{
            connection::{CltTest, CltTestSender, SvcTest, SvcTestSender},
            protocol::{CltTestProtocolManual, SvcTestProtocolManual},
        },
    };
    use log::info;
    use pyo3::{prelude::*, types::PyDict};
    use spin::Mutex;
    use std::num::NonZeroUsize;

    #[pyclass]
    struct PyLoggerCallback;
    #[pymethods]
    impl PyLoggerCallback {
        fn on_recv(&self, con_id: ConId, msg: Py<PyDict>) {
            info!("on_recv -> cond_id {}, msg: {}", con_id, msg)
        }
        fn on_sent(&self, con_id: ConId, msg: Py<PyDict>) {
            info!("on_sent -> cond_id {}, msg: {}", con_id, msg)
        }
    }

    #[test]
    fn test_clt2svc_macros() {
        setup::log::configure_compact(log::LevelFilter::Info);
        create_callback_for_messenger!(CltTestProtocolManual, CltTestProtocolManualCallback);
        create_callback_for_messenger!(SvcTestProtocolManual, SvcTestProtocolManualCallback);
        create_clt_sender!(CltManual, CltTestSender, CltTestProtocolManual, CltTestProtocolManualCallback, "unittest");
        create_svc_sender!(SvcManual, SvcTestSender, SvcTestProtocolManual, SvcTestProtocolManualCallback, "unittest");

        #[pymethods]
        impl CltManual {
            #[new]
            fn new(_py: Python<'_>, host: &str, callback: PyObject) -> Self {
                let protocol = CltTestProtocolManual::default();
                let callback = CltTestProtocolManualCallback::new_ref(callback);
                let clt = CltTest::connect(host, default_connect_timeout(), default_connect_retry_after(), callback, protocol, Some("py-clt"))
                    .unwrap()
                    .into_sender_with_spawned_recver();
                let con_id = clt.con_id().clone();
                Self {
                    sender: Mutex::new(Some(clt)),
                    con_id,
                    io_timeout: None,
                }
            }
        }
        #[pymethods]
        impl SvcManual {
            #[new]
            fn new(_py: Python<'_>, host: &str, callback: PyObject) -> Self {
                let protocol = SvcTestProtocolManual::default();
                let callback = SvcTestProtocolManualCallback::new_ref(callback);
                let max_connections = NonZeroUsize::new(1).unwrap();
                let clt = SvcTest::bind(host, max_connections, callback, protocol, Some("py-svc")).unwrap().into_sender_with_spawned_recver();
                let con_id = clt.con_id().clone();
                Self {
                    sender: Mutex::new(Some(clt)),
                    con_id,
                    io_timeout: None,
                }
            }
        }

        Python::with_gil(|py| {
            let callback = PyLoggerCallback {}.into_py(py);
            let addr = setup::net::rand_avail_addr_port();
            let svc = SvcManual::new(py, addr, callback.clone());
            info!("svc: {}", svc.__repr__(py));
            let clt = CltManual::new(py, addr, callback);

            info!("clt: {}", clt.__repr__(py));
            assert!(clt.is_connected(py, None));
            info!("svc: {}", svc.__repr__(py));
            assert!(svc.is_connected(py, None));

            let hbeat = PyDict::new(py);
            hbeat.set_item("ty", "H").unwrap();
            hbeat.set_item("text", "Blah").unwrap();
            let msg = PyDict::new(py);
            msg.set_item("HBeat", hbeat).unwrap();

            info!("msg: {}", msg); // "{'HBeat': {'ty': 'H', 'text': 'Blah'}}"
            clt.send(py, msg.into(), None).unwrap();

            svc.__exit__(py, None, None, None);
            info!("svc: {}", svc.__repr__(py));
            assert!(!svc.is_connected(py, None));

            clt.__del__(py);
            info!("clt: {}", clt.__repr__(py));
            assert!(!clt.is_connected(py, None));
        });
    }
}
