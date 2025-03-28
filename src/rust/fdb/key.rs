use std::ffi::CString;

use super::CKey;
use super::FDBLIB;

pub struct Key {
    key: *mut CKey,
}

#[macro_export]
macro_rules! create_key {
    ($($key:expr => $values:expr),* $(,)?) => {{
        let mut key = Key::new().unwrap();
        $(
            let _ = key.set($key, &$values);
        )*
        key
    }};
}

impl Key {
    pub fn new() -> Result<Self, String> {
        let mut key_ptr: *mut CKey = std::ptr::null_mut();
        let result = unsafe { (FDBLIB.fdb_new_key)(&mut key_ptr) };

        if result != 0 {
            return Err("Failed to create new key".into());
        }

        let key = Self { key: key_ptr };

        Ok(key)
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let param_c_str = CString::new(key).map_err(|e| e.to_string())?;
        let value_c_str = CString::new(value).map_err(|e| e.to_string())?;

        let result =
            unsafe { (FDBLIB.fdb_key_add)(self.key, param_c_str.as_ptr(), value_c_str.as_ptr()) };

        if result != 0 {
            return Err("Failed to add key/value".into());
        }
        Ok(())
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            (FDBLIB.fdb_delete_key)(self.key);
        }
    }
}
