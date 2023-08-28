//! Utils
use web_sys::console;

/// Timer
pub struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    /// Create a new timer
    #[inline]
    pub fn new(name: &'a str) -> Timer<'a> {
        console::time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        console::time_end_with_label(self.name);
    }
}
