use pyo3::prelude::*;

use crate::types::{PyFlow, PyKey, PyRow, PyView};

/// Populate a Python module with the complete public binding.
///
/// Public so the Rust integration tier can install the same module into an
/// embedded interpreter; the extension entry point delegates here too.
///
/// # Errors
///
/// Returns the first Python exception raised while registering a class,
/// function, or submodule.
pub fn add_to_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyKey>()?;
    module.add_class::<PyFlow>()?;
    module.add_class::<PyRow>()?;
    module.add_class::<PyView>()?;
    crate::settings::add_classes(module)?;
    crate::explore::add_classes(module)?;
    Ok(())
}

#[pymodule]
#[pyo3(gil_used = true)]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    add_to_module(module)
}
