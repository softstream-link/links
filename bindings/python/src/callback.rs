use links_nonblocking::prelude::asserted_short_name;
use links_nonblocking::prelude::ConId as ConIdRs;
use pyo3::{prelude::*, types::PyDict};
use serde::Serialize;
use serde_json::to_string;
use std::fmt::Debug;
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

/// An enum `pyclass` that is used in the python callback to indicate if the connection is an initiator or acceptor.
#[pyclass]
#[derive(Debug, Clone)]
pub enum ConType {
    Initiator,
    Acceptor,
}
impl Display for ConType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConType::Initiator => write!(f, "Initiator"),
            ConType::Acceptor => write!(f, "Acceptor"),
        }
    }
}

/// A `pyclass` structure that is used in the python callback to provide connection information.
#[pyclass]
#[derive(Debug, Clone)]
pub struct ConId {
    pub con_type: ConType,
    pub name: String,
    pub local: String,
    pub peer: String,
}
#[pymethods]
impl ConId {
    pub fn __repr__(&self) -> String {
        format!("{}", self)
    }
}
impl Display for ConId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.con_type {
            ConType::Initiator => write!(f, "{}({}@{}->{})", self.con_type, self.name, self.local, self.peer),
            ConType::Acceptor => write!(f, "{}({}@{}<-{})", self.con_type, self.name, self.local, self.peer),
        }
    }
}
impl From<&ConIdRs> for ConId {
    /// Convert from a rust ConIdRs to a python ConId
    fn from(value: &ConIdRs) -> Self {
        use ConIdRs::*;
        match value {
            Initiator { name, local, peer } => Self {
                con_type: ConType::Initiator,
                name: name.to_owned(),
                local: match local {
                    Some(local) => local.to_string(),
                    None => "pending".to_owned(),
                },
                peer: peer.to_string(),
            },
            Acceptor { name, local, peer } => Self {
                con_type: ConType::Acceptor,
                name: name.to_owned(),
                local: local.to_string(),
                peer: match peer {
                    Some(peer) => peer.to_string(),
                    None => "pending".to_owned(),
                },
            },
        }
    }
}
impl From<ConIdRs> for ConId {
    fn from(value: ConIdRs) -> Self {
        Self::from(&value)
    }
}

const ON_RECV: &str = "on_recv";
const ON_SENT: &str = "on_sent";

pub enum PyCallbackMethod {
    OnRecv,
    OnSent,
}
impl PyCallbackMethod {
    const fn as_str(&self) -> &'static str {
        match self {
            PyCallbackMethod::OnRecv => ON_RECV,
            PyCallbackMethod::OnSent => ON_SENT,
        }
    }
}

/// This is a helper structure that is used to propagate [links_nonblocking::prelude::CallbackRecvSend] to a [PyObject]
/// 'on_recv' and 'on_sent' methods.
#[derive(Debug)]
pub struct PyProxyCallback(PyObject);
impl PyProxyCallback {
    pub fn new(callback: PyObject) -> Self {
        Python::with_gil(|py| {
            callback.getattr(py, ON_RECV).unwrap_or_else(|_| panic!("callback must have {} method", ON_RECV));
            callback.getattr(py, ON_SENT).unwrap_or_else(|_| panic!("callback must have {} method", ON_SENT));
        });
        Self(callback)
    }
    pub fn new_ref(callback: PyObject) -> Arc<Self> {
        Arc::new(Self::new(callback))
    }

    pub fn issue_callback<O: Serialize + Debug>(&self, method: PyCallbackMethod, con_id: &ConIdRs, msg: &O) {
        let name = method.as_str();
        // convert msg to str
        let json = to_string(msg).unwrap_or_else(|_| panic!("serde_json::to_string failed to convert msg: {:?}", msg));
        let con_id = ConId::from(con_id);
        fn py_callback(obj: &PyObject, name: &str, con_id: &ConId, json: &String) -> PyResult<()> {
            Python::with_gil(|py| {
                let json_module = PyModule::import(py, "json")?;
                let dict = json_module.getattr("loads")?.call1((json,))?.extract::<Py<PyDict>>()?;

                let args = (con_id.clone(), dict);
                let kwargs = None;
                obj.call_method(py, name, args, kwargs)?;
                Ok(())
            })
        }

        match py_callback(&self.0, name, &con_id, &json) {
            Ok(_) => {}
            Err(err) => {
                let msg = err.to_string();
                if !msg.contains("import of builtins halted") {
                    // python is shutting down not point in logging this error
                    log::error!("{} failed '{}' on {} msg: {} err: {}", asserted_short_name!("PyProxyCallback", Self), name, con_id, json, err);
                }
            }
        }
    }
}
impl Display for PyProxyCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", asserted_short_name!("PyProxyCallback", Self))
    }
}

#[macro_export]
macro_rules! create_callback_for_messenger(
    ($name:ident, $protocol:ident) => {
        #[derive(Debug)]
        struct $name($crate::prelude::PyProxyCallback);
        impl $name {
            pub fn new_ref(callback: pyo3::PyObject) -> std::sync::Arc<Self> {
                std::sync::Arc::new(Self($crate::prelude::PyProxyCallback::new(callback)))
            }
        }
        impl $crate::prelude::CallbackRecv<$protocol> for $name {
            fn on_recv(&self, con_id: &$crate::prelude::ConIdRs, msg: &<$protocol as $crate::prelude::Messenger>::RecvT) {
                self.0.issue_callback($crate::prelude::PyCallbackMethod::OnRecv, con_id, msg)
            }
        }
        impl $crate::prelude::CallbackSend<$protocol> for $name {
            fn on_sent(&self, con_id: &$crate::prelude::ConIdRs, msg: &<$protocol as $crate::prelude::Messenger>::SendT) {
                self.0.issue_callback($crate::prelude::PyCallbackMethod::OnSent, con_id, msg);
            }
        }
        impl $crate::prelude::CallbackRecvSend<$protocol> for $name {}
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f,  std::stringify!($name))
            }
        }
    }
);

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use links_nonblocking::prelude::{
        setup::{
            self,
            model::{CltTestMsgDebug, SvcTestMsgDebug},
        },
        ConId as ConIdRs,
    };
    use log::info;
    use pyo3::{prelude::*, types::PyDict};

    #[test]
    #[should_panic(expected = "callback must have on_recv method")]
    fn test_py_callback_invalid() {
        let invalid_python_callback = Python::with_gil(|py| {
            let any: PyObject = py.None();
            any
        });

        use links_nonblocking::unittest::setup::protocol::CltTestProtocolManual;
        create_callback_for_messenger!(CltTestProtocolManualCallback, CltTestProtocolManual);
        let _ = CltTestProtocolManualCallback::new_ref(invalid_python_callback);
    }

    #[test]
    fn test_py_callback_valid() {
        setup::log::configure();
        #[pyclass]
        struct ValidPythonCallback;
        #[pymethods]
        impl ValidPythonCallback {
            fn on_recv(&self, con_id: ConId, msg: Py<PyDict>) {
                info!("on_recv -> cond_id {}, msg: {}", con_id, msg)
            }
            fn on_sent(&self, con_id: ConId, msg: Py<PyDict>) {
                info!("on_sent -> cond_id {}, msg: {}", con_id, msg)
            }
        }

        let valid_python_callback = Python::with_gil(|py| {
            let any: PyObject = ValidPythonCallback {}.into_py(py);
            any
        });

        use links_nonblocking::unittest::setup::protocol::CltTestProtocolManual;
        create_callback_for_messenger!(CltTestProtocolManualCallback, CltTestProtocolManual);
        let callback = CltTestProtocolManualCallback::new_ref(valid_python_callback);
        let con_id = ConIdRs::clt(Some("clt"), None, "127.0.0.1:8080");
        let msg = CltTestMsgDebug::default().into();
        callback.on_sent(&con_id, &msg);
        let msg = SvcTestMsgDebug::default().into();
        callback.on_recv(&con_id, &msg);
    }
}
