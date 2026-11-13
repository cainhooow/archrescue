//! Mode-aware presentation of typed paths and embedded diagnostic details.

pub use archrescue_core::presentation::{path as friendly, text};

pub fn display(path: &std::path::Path, technician: bool) -> String {
    if technician {
        path.display().to_string()
    } else {
        friendly(path)
    }
}
