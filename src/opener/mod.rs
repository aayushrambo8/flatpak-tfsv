pub mod detection;
pub mod launcher;

pub use detection::detect_opener;
pub use launcher::{launch_file, launch_file_with_terminal};
