use ndarray::{ArrayD};
use ::nrrd::{Nrrd, NrrdData, read_file, parse_list};
use numpy::{IntoPyArray};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict};
use std::path::Path;


macro_rules! into_pyarray {
    ($data:expr, $sizes:expr, $py:expr) => {
        unsafe { ArrayD::from_shape_vec_unchecked($sizes, $data).into_pyarray_bound($py).into() }
    };
}


#[pyclass(name = "Nrrd")]
#[repr(transparent)]
pub struct PyNrrd {
    pub nrrd: Nrrd,
}

#[pymethods]
impl PyNrrd {
    #[new]
    fn new(path: &str) -> Self {
        let nrrd = Nrrd::from_file(Path::new(path));

        PyNrrd { nrrd }
    }

    #[getter]
    fn metadata<'a>(&self, py: Python<'a>) -> Bound<'a, PyDict> {
        //XXX do I really need to copy the HashMap?
        self.nrrd.metadata.clone().into_py_dict_bound(py).into()
    }

    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> PyObject {

        let sizes = self.nrrd.sizes();

        match &self.nrrd.data {
            NrrdData::I8(data)  => into_pyarray!(data.clone(), sizes, py),
            NrrdData::U8(data)  => into_pyarray!(data.clone(), sizes, py),
            NrrdData::I16(data) => into_pyarray!(data.clone(), sizes, py),
            NrrdData::U16(data) => into_pyarray!(data.clone(), sizes, py),
            NrrdData::I32(data) => into_pyarray!(data.clone(), sizes, py),
            NrrdData::U32(data) => into_pyarray!(data.clone(), sizes, py),
            NrrdData::I64(data) => into_pyarray!(data.clone(), sizes, py),
            NrrdData::U64(data) => into_pyarray!(data.clone(), sizes, py),
            NrrdData::F32(data) => into_pyarray!(data.clone(), sizes, py),
            NrrdData::F64(data) => into_pyarray!(data.clone(), sizes, py),
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
fn nrrd<'py>(m: &Bound<'py, PyModule>) -> PyResult<()> {
    m.add_class::<PyNrrd>()?;

    #[pyfn(m)]
    #[pyo3(name = "read")]
    fn read<'py>(
        py: Python<'py>,
        path: &str,
    ) -> (PyObject, Bound<'py, PyDict>) {

        let (metadata, data) = read_file(Path::new(path));

        let pyheader = metadata.clone().into_py_dict_bound(py).into();

        let sizes = parse_list::<usize>(metadata["sizes"].as_str());

        let pydata = match data {
            NrrdData::I8(data)  => into_pyarray!(data, sizes, py),
            NrrdData::U8(data)  => into_pyarray!(data, sizes, py),
            NrrdData::I16(data) => into_pyarray!(data, sizes, py),
            NrrdData::U16(data) => into_pyarray!(data, sizes, py),
            NrrdData::I32(data) => into_pyarray!(data, sizes, py),
            NrrdData::U32(data) => into_pyarray!(data, sizes, py),
            NrrdData::I64(data) => into_pyarray!(data, sizes, py),
            NrrdData::U64(data) => into_pyarray!(data, sizes, py),
            NrrdData::F32(data) => into_pyarray!(data, sizes, py),
            NrrdData::F64(data) => into_pyarray!(data, sizes, py),
        };

        (pydata, pyheader)
    }

    Ok(())
}
