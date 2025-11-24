//! Internal macro helpers shared by the `PyO3` bindings.

macro_rules! define_conversion_pair {
    (
        from $from_fn:ident($from_arg:ident : $from_ty:ty) -> $from_err:ty { $from_body:expr };
        to $to_fn:ident($to_arg:ident : $to_ty:ty) -> $to_ret:ty, $to_err:ty { $to_body:expr }
    ) => {
        fn $from_fn($from_arg: $from_ty) -> Result<TeiDocument, $from_err> {
            $from_body
        }

        fn $to_fn($to_arg: $to_ty) -> Result<$to_ret, $to_err> {
            $to_body
        }
    };
}

macro_rules! define_py_from_error_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : $ty:ty) -> Document using $inner:ident, $fmt:expr) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: $ty) -> PyResult<Document> {
            $inner($param)
                .map(Document::from)
                .map_err(|error| PyValueError::new_err(format!($fmt, error = error)))
        }
    };
}

macro_rules! define_py_to_error_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : &$ty:ty) -> $ret:ty => $inner:ident, $fmt:expr) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: &$ty) -> PyResult<$ret> {
            $inner($param).map_err(|error| PyValueError::new_err(format!($fmt, error = error)))
        }
    };
}

macro_rules! define_py_from_result_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : $ty:ty) -> Document using $inner:ident) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: $ty) -> PyResult<Document> {
            wrap_tei_result($inner($param)).map(Document::from)
        }
    };
}

macro_rules! define_py_to_result_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : &$ty:ty) -> $ret:ty => $inner:ident) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: &$ty) -> PyResult<$ret> {
            wrap_tei_result($inner($param))
        }
    };
}

pub(crate) use define_conversion_pair;
pub(crate) use define_py_from_error_wrapper;
pub(crate) use define_py_from_result_wrapper;
pub(crate) use define_py_to_error_wrapper;
pub(crate) use define_py_to_result_wrapper;
