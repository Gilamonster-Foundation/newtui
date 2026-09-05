use newtui_core::{Flow, Key, Row, View};
use pyo3::prelude::*;

#[pyclass(name = "Key", frozen, eq, skip_from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PyKey {
    pub(crate) inner: Key,
}

#[pymethods]
impl PyKey {
    #[classattr]
    const UP: Self = Self { inner: Key::Up };
    #[classattr]
    const DOWN: Self = Self { inner: Key::Down };
    #[classattr]
    const LEFT: Self = Self { inner: Key::Left };
    #[classattr]
    const RIGHT: Self = Self { inner: Key::Right };
    #[classattr]
    const ENTER: Self = Self { inner: Key::Enter };
    #[classattr]
    const ESC: Self = Self { inner: Key::Esc };

    #[staticmethod]
    fn character(value: char) -> Self {
        Self {
            inner: Key::Char(value),
        }
    }

    #[staticmethod]
    fn control(value: char) -> Self {
        Self {
            inner: Key::Ctrl(value),
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __repr__(&self) -> String {
        format!("Key({})", self.inner)
    }
}

impl From<Key> for PyKey {
    fn from(inner: Key) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "Flow", frozen, eq, skip_from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PyFlow {
    pub(crate) inner: Flow,
}

#[pymethods]
impl PyFlow {
    #[staticmethod]
    fn stay() -> Self {
        Self { inner: Flow::Stay }
    }

    #[staticmethod]
    fn close(applied: bool) -> Self {
        Self {
            inner: Flow::Close(applied),
        }
    }

    #[getter]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn closed(&self) -> bool {
        matches!(self.inner, Flow::Close(_))
    }

    #[getter]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn applied(&self) -> bool {
        matches!(self.inner, Flow::Close(true))
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __repr__(&self) -> String {
        match self.inner {
            Flow::Stay => "Flow.stay()".to_string(),
            Flow::Close(applied) => format!("Flow.close({applied})"),
        }
    }
}

impl From<Flow> for PyFlow {
    fn from(inner: Flow) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "Row", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub(crate) struct PyRow {
    pub(crate) inner: Row,
}

#[pymethods]
impl PyRow {
    #[new]
    #[pyo3(signature = (label, value, note = "", selected = false, adjustable = false))]
    fn new(label: String, value: String, note: &str, selected: bool, adjustable: bool) -> Self {
        Self {
            inner: Row {
                label,
                value,
                note: note.to_string(),
                selected,
                adjustable,
            },
        }
    }

    #[getter]
    fn label(&self) -> &str {
        &self.inner.label
    }

    #[getter]
    fn value(&self) -> &str {
        &self.inner.value
    }

    #[getter]
    fn note(&self) -> &str {
        &self.inner.note
    }

    #[getter]
    fn selected(&self) -> bool {
        self.inner.selected
    }

    #[getter]
    fn adjustable(&self) -> bool {
        self.inner.adjustable
    }
}

impl From<Row> for PyRow {
    fn from(inner: Row) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "View", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub(crate) struct PyView {
    pub(crate) inner: View,
}

#[pymethods]
impl PyView {
    #[new]
    #[pyo3(signature = (title, rows, footer = ""))]
    fn new(title: String, rows: Vec<Py<PyRow>>, footer: &str, py: Python<'_>) -> Self {
        Self {
            inner: View {
                title,
                rows: rows
                    .into_iter()
                    .map(|row| row.borrow(py).inner.clone())
                    .collect(),
                footer: footer.to_string(),
            },
        }
    }

    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }

    #[getter]
    fn rows(&self) -> Vec<PyRow> {
        self.inner.rows.iter().cloned().map(PyRow::from).collect()
    }

    #[getter]
    fn footer(&self) -> &str {
        &self.inner.footer
    }
}

impl From<View> for PyView {
    fn from(inner: View) -> Self {
        Self { inner }
    }
}
