mod motion;

#[cfg(target_os = "windows")]
pub use motion::install_motion;
