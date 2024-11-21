use ndarray::{ArrayView, IxDyn};
use nrrd::{Nrrd as _Nrrd, NrrdData};
use numpy::{PyArray, ToPyArray};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict};
use std::path::Path;

#[pyclass]
#[repr(transparent)]
pub struct Nrrd {
    pub nrrd: _Nrrd,
}

#[pymethods]
impl Nrrd {
    #[new]
    fn new(path: &str) -> Self {
        let nrrd = _Nrrd::from_file(Path::new(path));

        Nrrd { nrrd }
    }

    #[getter]
    fn metadata(&self, py: Python) -> Py<PyDict> {
        //XXX do I really need to copy the HashMap?
        let mut ret = std::collections::HashMap::new();
        ret.extend(self.nrrd.metadata.iter());
        ret.into_py_dict(py).into()
    }

    #[getter]
    fn data(&self, py: Python) -> PyObject {

        match &self.nrrd.data {
            NrrdData::I8(data)  => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::U8(data)  => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::I16(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::U16(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::I32(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::U32(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::I64(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::U64(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::F32(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into(),
            NrrdData::F64(data) => ArrayView::from_shape(self.nrrd.sizes(), &data).unwrap().to_pyarray(py).into()

        }


        //unsafe {
        //    let array = ArrayView::from_shape_ptr(self.nrrd.sizes(), self.nrrd.data.as_ptr());
        //    array.to_pyarray(py).into()
        //}
    }

    //fn sum(&self) -> f64 {
    //    self.nrrd.data.iter().sum::<f64>()
    //}
}

#[pymodule]
fn nrrd(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Nrrd>()?;
    Ok(())
}
