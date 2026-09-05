use std::cell::RefCell;
use std::rc::Rc;

use newtui_core::{
    properties, Component, Explorer, Fingerprint, Flow, Key, Property, Report, Row, View,
};
use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;

use crate::types::{PyFlow, PyKey, PyView};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallbackFailure {
    method: &'static str,
    detail: String,
    path: Vec<Key>,
}

#[pyclass(name = "CallbackError", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub(crate) struct PyCallbackError {
    #[pyo3(get)]
    method: &'static str,
    #[pyo3(get)]
    detail: String,
    path: Vec<PyKey>,
}

#[pymethods]
impl PyCallbackError {
    #[getter]
    fn path(&self) -> Vec<PyKey> {
        self.path.clone()
    }
}

impl From<&CallbackFailure> for PyCallbackError {
    fn from(failure: &CallbackFailure) -> Self {
        Self {
            method: failure.method,
            detail: failure.detail.clone(),
            path: failure.path.iter().copied().map(PyKey::from).collect(),
        }
    }
}

#[pyclass(name = "PropertySet", frozen)]
pub(crate) struct PyPropertySet;

#[pyfunction]
pub(crate) fn standard() -> PyPropertySet {
    PyPropertySet
}

fn standard_properties() -> Vec<Box<dyn Property>> {
    vec![
        Box::new(properties::selection_is_always_in_range()),
        Box::new(properties::escape_always_closes_without_applying()),
        Box::new(properties::only_adjustable_rows_move()),
    ]
}

struct PythonComponent<'py> {
    py: Python<'py>,
    object: Option<Py<PyAny>>,
    failures: Rc<RefCell<Vec<CallbackFailure>>>,
    path: RefCell<Vec<Key>>,
    faulted: RefCell<bool>,
}

impl<'py> PythonComponent<'py> {
    fn new(
        py: Python<'py>,
        factory: &Bound<'py, PyAny>,
        failures: Rc<RefCell<Vec<CallbackFailure>>>,
    ) -> Self {
        match factory.call0() {
            Ok(object) => Self {
                py,
                object: Some(object.unbind()),
                failures,
                path: RefCell::new(Vec::new()),
                faulted: RefCell::new(false),
            },
            Err(error) => {
                let failure = CallbackFailure {
                    method: "factory",
                    detail: error.to_string(),
                    path: Vec::new(),
                };
                Self::record(&failures, failure);
                Self {
                    py,
                    object: None,
                    failures,
                    path: RefCell::new(Vec::new()),
                    faulted: RefCell::new(true),
                }
            }
        }
    }

    fn record(failures: &Rc<RefCell<Vec<CallbackFailure>>>, failure: CallbackFailure) {
        let mut failures = failures.borrow_mut();
        if !failures.contains(&failure) {
            failures.push(failure);
        }
    }

    fn fail(&self, method: &'static str, error: &PyErr) {
        *self.faulted.borrow_mut() = true;
        Self::record(
            &self.failures,
            CallbackFailure {
                method,
                detail: error.to_string(),
                path: self.path.borrow().clone(),
            },
        );
    }

    fn error_view(&self) -> View {
        let detail = self.failures.borrow().last().map_or_else(
            || "Python callback failed".to_string(),
            |failure| failure.detail.clone(),
        );
        View::titled("python component error")
            .row(Row::new("callback", "failed").note(detail).selected())
            .footer("the exception is recorded in Report.errors")
    }
}

impl Component for PythonComponent<'_> {
    fn handle(&mut self, key: Key) -> Flow {
        self.path.get_mut().push(key);
        if *self.faulted.borrow() {
            return Flow::Close(false);
        }
        let Some(object) = &self.object else {
            return Flow::Close(false);
        };
        let result = object
            .call_method1(self.py, "handle", (PyKey::from(key),))
            .and_then(|value| {
                Ok(value
                    .extract::<PyRef<'_, PyFlow>>(self.py)
                    .map(|flow| flow.inner)?)
            });
        match result {
            Ok(flow) => flow,
            Err(error) => {
                self.fail("handle", &error);
                Flow::Close(false)
            }
        }
    }

    fn view(&self) -> View {
        if *self.faulted.borrow() {
            return self.error_view();
        }
        let Some(object) = &self.object else {
            return self.error_view();
        };
        let result = object.call_method0(self.py, "view").and_then(|value| {
            Ok(value
                .extract::<PyRef<'_, PyView>>(self.py)
                .map(|view| view.inner.clone())?)
        });
        match result {
            Ok(view) => view,
            Err(error) => {
                self.fail("view", &error);
                self.error_view()
            }
        }
    }

    fn fingerprint(&self) -> Fingerprint {
        if *self.faulted.borrow() {
            return Fingerprint::of_view(&self.error_view()).and("python callback failed");
        }
        let Some(object) = &self.object else {
            return Fingerprint::of_view(&self.error_view()).and("python factory failed");
        };
        match object.call_method0(self.py, "fingerprint") {
            Ok(value) => match value.extract::<String>(self.py) {
                Ok(value) => Fingerprint::of(value),
                Err(error) => {
                    self.fail("fingerprint", &error);
                    Fingerprint::of_view(&self.error_view()).and("python fingerprint failed")
                }
            },
            Err(error) if error.is_instance_of::<PyAttributeError>(self.py) => {
                Fingerprint::of_view(&self.view())
            }
            Err(error) => {
                self.fail("fingerprint", &error);
                Fingerprint::of_view(&self.error_view()).and("python fingerprint failed")
            }
        }
    }
}

#[pyclass(name = "Report", frozen)]
pub(crate) struct PyReport {
    report: Report,
    failures: Vec<CallbackFailure>,
}

#[pymethods]
impl PyReport {
    #[getter]
    fn states(&self) -> usize {
        self.report.states
    }

    #[getter]
    fn transitions(&self) -> usize {
        self.report.transitions
    }

    #[getter]
    fn exhausted(&self) -> bool {
        self.report.exhausted
    }

    #[getter]
    fn is_clean(&self) -> bool {
        self.failures.is_empty() && self.report.is_clean()
    }

    #[getter]
    fn errors(&self) -> Vec<PyCallbackError> {
        self.failures.iter().map(PyCallbackError::from).collect()
    }

    fn __str__(&self) -> String {
        let mut rendered = self.report.to_string();
        if !self.failures.is_empty() {
            use std::fmt::Write;
            let _ = write!(
                rendered,
                "\n{} Python callback errors:",
                self.failures.len()
            );
            for failure in &self.failures {
                let path = failure
                    .path
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = write!(
                    rendered,
                    "\n{} after {}: {}",
                    failure.method,
                    if path.is_empty() { "(initial)" } else { &path },
                    failure.detail
                );
            }
        }
        rendered
    }
}

#[pyfunction]
pub(crate) fn explore(
    py: Python<'_>,
    factory: &Bound<'_, PyAny>,
    _properties: PyRef<'_, PyPropertySet>,
) -> PyReport {
    // The callbacks ARE Python, so releasing the GIL would make the component
    // impossible to call. One attachment spans the complete replay walk; the
    // catalogue reports the measured cost instead of hiding it behind an
    // unavailable `allow_threads` escape hatch.
    let failures = Rc::new(RefCell::new(Vec::new()));
    let owned = standard_properties();
    let properties: Vec<&dyn Property> = owned.iter().map(AsRef::as_ref).collect();
    let report = Explorer::new(Key::navigation()).explore(
        || PythonComponent::new(py, factory, Rc::clone(&failures)),
        &properties,
    );
    let failures = failures.borrow().clone();
    PyReport { report, failures }
}

pub(crate) fn add_classes(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyCallbackError>()?;
    module.add_class::<PyPropertySet>()?;
    module.add_class::<PyReport>()?;
    module.add_function(wrap_pyfunction!(explore, module)?)?;

    let properties = PyModule::new(module.py(), "properties")?;
    properties.add_function(wrap_pyfunction!(standard, &properties)?)?;
    module.add_submodule(&properties)?;
    Ok(())
}
