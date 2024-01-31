#[macro_export]
macro_rules! create_register_atexit {
    () => {
        #[pyo3::prelude::pyfunction]
        fn atexit_register_hook(_py: pyo3::prelude::Python<'_>) {
            _py.allow_threads(move || {
                log::info!("shutting down DEFAULT_POOL_HANDLER");
                links_nonblocking::prelude::DEFAULT_POLL_HANDLER.shutdown(None);
                log::info!("shutting down DEFAULT_HBEAT_HANDLER");
                links_nonblocking::prelude::DEFAULT_HBEAT_HANDLER.clear();
                std::thread::sleep(std::time::Duration::from_millis(100));
            });
        }
        fn register_atexit() -> pyo3::prelude::PyResult<()> {
            pyo3::prelude::Python::with_gil(|py| {
                let fn_register: pyo3::prelude::Py<pyo3::prelude::PyAny> = pyo3::prelude::PyModule::import(py, "atexit")?.getattr("register")?.into();
                let fn_atexit_register_hook = pyo3::prelude::wrap_pyfunction!(atexit_register_hook, py)?;
                fn_register.call1(py, (fn_atexit_register_hook,))?;
                Ok(())
            })
        }
    };
}
