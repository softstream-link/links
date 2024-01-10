#[macro_export]
macro_rules! enter_dunder(
    ($name:ident) => {
        // struct CltManual;
        #[pyo3::pymethods]
        impl $name{
            fn __enter__(slf: pyo3::Py<Self>) -> pyo3::Py<Self> {
                slf
            }
        }
    }
);

#[cfg(test)]
mod test {

    #[test]
    fn test_enter_dunder() {
        #[pyo3::pyclass]
        struct CltManual;
        enter_dunder!(CltManual);
    }
}
