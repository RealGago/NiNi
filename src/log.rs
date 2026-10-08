use std::io::Write;

pub fn log(msg: &str) {
    if let Some(dir) = dirs::cache_dir() {
        let dir = dir.join("nini");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("nini.log"))
        {
            let _ = writeln!(f, "{}", msg);
        }
    }
}

#[macro_export]
macro_rules! nlog {
    ($($arg:tt)*) => { $crate::log::log(&format!($($arg)*)) };
}
