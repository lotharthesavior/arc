//! Restore process environment in the integration-test executable, including on assertion panic.
//! Tests sharing a key must still be serialized; separate integration executables have separate envs.
pub struct EnvGuard(Vec<(String, Option<std::ffi::OsString>)>);
impl EnvGuard {
    pub fn new(values: &[(&str, Option<&str>)]) -> Self {
        let previous = values
            .iter()
            .map(|(key, _)| ((*key).to_owned(), std::env::var_os(key)))
            .collect();
        for (key, value) in values {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self(previous)
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}
