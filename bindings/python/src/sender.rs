#[macro_export]
macro_rules! create_clt_sender(
    ($name:ident, $sender:ident, $protocol:ident, $callback:ident) => {
        $crate::create_struct!($name, $sender, $protocol, $callback);
        $crate::send!($name);
        $crate::clt_is_connected!($name, $sender, $protocol);
        $crate::clt__repr__!($name);
        $crate::__enter__!($name);
        $crate::__exit__!($name);
        $crate::__del__!($name);
    }
);

#[macro_export]
macro_rules! create_svc_sender(
    ($name:ident, $sender:ident, $protocol:ident, $callback:ident) => {
        $crate::create_struct!($name, $sender, $protocol, $callback);
        $crate::send!($name);
        $crate::svc_is_connected!($name, $sender, $protocol);
        $crate::svc__repr__!($name);
        $crate::__enter__!($name);
        $crate::__exit__!($name);
        $crate::__del__!($name);
    }
);

#[macro_export]
macro_rules! send(
    ($name:ident) => {
        #[pyo3::pymethods]
        impl $name{
            fn send(&mut self, _py: pyo3::Python<'_>, msg: pyo3::Py<pyo3::types::PyDict>, io_timeout: Option<f64>) -> pyo3::PyResult<()> {
                let io_timeout = $crate::timeout_selector(io_timeout, self.io_timeout);
                let json_module = pyo3::types::PyModule::import(_py, "json")?;
                let json: String = json_module.getattr("dumps")?.call1((msg,))?.extract()?;
                let mut msg = serde_json::from_str(json.as_str()).unwrap();

                _py.allow_threads(move || match self.sender.send_busywait_timeout(&mut msg, io_timeout)? {
                    SendStatus::Completed => Ok(()),
                    SendStatus::WouldBlock => Err(Error::new(ErrorKind::WouldBlock, format!("Message not delivered due timeout: {:?}, msg: {}", io_timeout, json)).into()),
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
            #[doc = concat!("[`", stringify!($name), ".is_connected`] returns status of connection for [`", stringify!($sender) , "`] according to the [`", stringify!($protocol) ,"`] protocol implementation")]
            fn is_connected(&self, _py: pyo3::Python<'_>, io_timeout: Option<f64>) -> bool {
                let io_timeout = $crate::timeout_selector(io_timeout, self.io_timeout);
                _py.allow_threads(move || self.sender.is_connected_busywait_timeout(io_timeout))
            }
        }
    }
);
#[macro_export]
macro_rules! svc_is_connected(
    ($name:ident, $sender:ident, $protocol:ident) => {
        #[pyo3::pymethods]
        impl $name{
            #[doc = concat!("[`", stringify!($name), ".is_connected`] returns status of the next connection in the pool of [`", stringify!($sender) , "`] according to the [`", stringify!($protocol) ,"`] protocol implementation")]
            fn is_connected(&mut self, _py: Python<'_>, io_timeout: Option<f64>) -> bool {
                let io_timeout = timeout_selector(io_timeout, self.io_timeout);
                _py.allow_threads(move || self.sender.is_next_connected_busywait_timeout(io_timeout))
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
                _py.allow_threads(move || {
                    let is_connected = self.sender.is_connected();
                    format!("{}({}, is_connected: {})", stringify!($name), self.sender.con_id(), is_connected)
                })
            }
        }
    }
);
#[macro_export]
macro_rules! svc__repr__(
    ($name:ident) => {
        #[pyo3::pymethods]
        impl $name{
            fn __repr__(&mut self, _py: pyo3::Python<'_>) -> String {
                _py.allow_threads(move || {
                    let is_connected = self.sender.is_next_connected();
                    if !is_connected {
                        format!("{}({}, is_connected: {})", stringify!($name), self.sender.con_id(), is_connected)
                    } else {
                        let num = self.sender.len();
                        let max = self.sender.max_connections();
                        let connections = self.sender.iter().map(|(_, s)| format!("[{}, is_connected: {}]", s.con_id(), s.is_connected())).collect::<Vec<_>>().join(",");
                        format!("{}(#{} of max {} {})", stringify!($name), num, max, connections)
                    }
                })
            }
        }
    }
);

#[macro_export]
macro_rules! create_struct(
    ($name:ident, $sender:ident, $protocol:ident, $callback:ident) => {
        #[doc = concat!("[`", stringify!($name), "`] is a python extension module for [`", stringify!($sender), "`] sender, implementing [`", stringify!($protocol) ,"`] protocol")]
        #[pyo3::pyclass]
        pub struct $name {
            sender: $sender<$protocol, $callback>,
            io_timeout: Option<f64>,
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
        /// Calls [`Shutdown::shutdown`] on the sender.
        #[pyo3::pymethods]
        impl $name{
            fn __exit__(&mut self, _py: pyo3::Python<'_>, _exc_type: Option<&pyo3::PyAny>, _exc_value: Option<&pyo3::PyAny>, _traceback: Option<&pyo3::PyAny>) {
                self.sender.shutdown()
            }
        }
    }
);
#[macro_export]
macro_rules! __del__(
    ($name:ident) => {
        /// Calls [`Shutdown::shutdown`] on the sender.
        #[pyo3::pymethods]
        impl $name{
            fn __del__(&mut self) {
                self.sender.shutdown()
            }
        }
    }
);

#[cfg(test)]
mod test {

    #[test]
    fn test___enter__() {
        #[pyo3::pyclass]
        struct CltManual;
        __enter__!(CltManual);
    }
}
