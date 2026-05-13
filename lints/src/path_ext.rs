use std::path::Path;

pub(crate) trait PathExt {
    /// True when the path is a real local source file (not pulled in from
    /// `~/.cargo`, `~/.rustup`, the toolchain `rustlib`, or a synthetic
    /// `<...>` filename like `<built-in>`).
    fn is_local_source(&self) -> bool;
}

impl PathExt for Path {
    fn is_local_source(&self) -> bool {
        let s = self.to_string_lossy();
        !s.contains("/.cargo/")
            && !s.contains("/.rustup/")
            && !s.contains("/rustlib/")
            && !s.starts_with("<")
    }
}
