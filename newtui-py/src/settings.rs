use newtui_core::components::settings_panel::{
    Backend, Choice, Model, Setting, SettingChange, SettingsIntent, SettingsPanel, SettingsSeed,
};
use newtui_core::Component;
use pyo3::prelude::*;

use crate::types::{PyFlow, PyKey, PyView};

#[pyclass(name = "Choice", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyChoice {
    inner: Choice,
}

#[pymethods]
impl PyChoice {
    #[new]
    fn new(value: String, note: String) -> Self {
        Self {
            inner: Choice::new(value, note),
        }
    }
}

#[pyclass(name = "Setting", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PySetting {
    inner: Setting,
}

#[pymethods]
impl PySetting {
    #[staticmethod]
    fn choice(
        py: Python<'_>,
        key: String,
        label: String,
        current: String,
        choices: Vec<Py<PyChoice>>,
    ) -> Self {
        Self {
            inner: Setting::choice(
                key,
                label,
                current,
                choices
                    .into_iter()
                    .map(|choice| choice.borrow(py).inner.clone())
                    .collect(),
            ),
        }
    }

    #[staticmethod]
    fn number(
        key: String,
        label: String,
        current: String,
        release: String,
        min: usize,
        max: usize,
    ) -> Self {
        Self {
            inner: Setting::number(key, label, current, release, min, max),
        }
    }

    #[staticmethod]
    fn fixed(key: String, label: String, current: String, note: String) -> Self {
        Self {
            inner: Setting::fixed(key, label, current, note),
        }
    }
}

#[pyclass(name = "SettingChange", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PySettingChange {
    #[pyo3(get)]
    key: String,
    #[pyo3(get)]
    value: String,
}

impl From<&SettingChange> for PySettingChange {
    fn from(change: &SettingChange) -> Self {
        Self {
            key: change.key.clone(),
            value: change.value.clone(),
        }
    }
}

#[pyclass(name = "SettingsIntent", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PySettingsIntent {
    #[pyo3(get)]
    kind: &'static str,
    changes: Vec<PySettingChange>,
    #[pyo3(get)]
    model: Option<String>,
}

#[pymethods]
impl PySettingsIntent {
    #[getter]
    fn changes(&self) -> Vec<PySettingChange> {
        self.changes.clone()
    }
}

impl From<&SettingsIntent> for PySettingsIntent {
    fn from(intent: &SettingsIntent) -> Self {
        let (kind, changes, model) = match intent {
            SettingsIntent::Apply { changes, model } => ("apply", changes, model),
            SettingsIntent::OpenBackends { changes, model } => ("open_backends", changes, model),
        };
        Self {
            kind,
            changes: changes.iter().map(PySettingChange::from).collect(),
            model: model.clone(),
        }
    }
}

#[pyclass(name = "SettingsPanel")]
pub(crate) struct PySettingsPanel {
    inner: SettingsPanel,
}

#[pymethods]
impl PySettingsPanel {
    // PyO3 borrows a frozen class receiver from Python; taking this tiny value
    // by reference is the generated method seam, not a Rust call-site choice.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn handle(&mut self, key: &PyKey) -> PyFlow {
        self.inner.handle(key.inner).into()
    }

    fn view(&self) -> PyView {
        self.inner.view().into()
    }

    fn intent(&self) -> Option<PySettingsIntent> {
        self.inner.intent().map(PySettingsIntent::from)
    }
}

#[pyfunction]
#[pyo3(signature = (*, settings = None, backend = None, models = None, current_model = None))]
pub(crate) fn settings_panel(
    py: Python<'_>,
    settings: Option<Vec<Py<PySetting>>>,
    backend: Option<String>,
    models: Option<Vec<String>>,
    current_model: Option<String>,
) -> PySettingsPanel {
    let settings = settings.map_or_else(Vec::new, |settings| {
        settings
            .into_iter()
            .map(|setting| setting.borrow(py).inner.clone())
            .collect()
    });
    let current = current_model
        .or_else(|| models.as_ref().and_then(|models| models.first().cloned()))
        .unwrap_or_default();
    let choices = models.map(|models| {
        models
            .into_iter()
            .map(|name| Choice::new(name, "served by the active backend"))
            .collect()
    });
    PySettingsPanel {
        inner: SettingsPanel::new(SettingsSeed::new(
            settings,
            Model::new(current, choices),
            Backend::new(backend),
        )),
    }
}

pub(crate) fn add_classes(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyChoice>()?;
    module.add_class::<PySetting>()?;
    module.add_class::<PySettingChange>()?;
    module.add_class::<PySettingsIntent>()?;
    module.add_class::<PySettingsPanel>()?;
    module.add_function(wrap_pyfunction!(settings_panel, module)?)?;
    Ok(())
}
