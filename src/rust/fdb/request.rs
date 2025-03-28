use super::FDBLIB;
use libc::c_char;
use serde_json::Value;
use std::ffi::CString;

#[repr(C)]
pub struct CRequest {
    _empty: [u8; 0],
}

pub struct Request {
    request: *mut CRequest,
}

impl Request {
    pub fn new() -> Result<Self, String> {
        let mut request_ptr: *mut CRequest = std::ptr::null_mut();
        let result = unsafe { (FDBLIB.fdb_new_request)(&mut request_ptr) };
        if result != 0 {
            return Err("Failed to create new request".into());
        }
        let request = Self {
            request: request_ptr,
        };

        Ok(request)
    }

    pub fn set<'a, C, T>(&mut self, key: &'a str, values: C) -> Result<(), String>
    where
        C: AsRef<[T]>,
        T: AsRef<str> + 'a,
    {
        let values_slice = values.as_ref();
        let key_cstr = CString::new(key).map_err(|_| "Failed to create CString for key")?;

        let cvals: Vec<CString> = values_slice
            .iter()
            .map(|val| CString::new(val.as_ref()).map_err(|_| "Failed to create CString for value"))
            .collect::<Result<Vec<_>, _>>()?;

        let cvals_ptrs: Vec<*const c_char> = cvals.iter().map(|cstr| cstr.as_ptr()).collect();

        let result = unsafe {
            (FDBLIB.fdb_request_add)(
                self.request,
                key_cstr.as_ptr(),
                cvals_ptrs.as_ptr(),
                values_slice.len(),
            )
        };

        if result != 0 {
            return Err(format!("Failed to add values for key '{}'", key));
        }

        Ok(())
    }

    pub fn as_ptr(&self) -> *mut CRequest {
        self.request
    }

    pub fn from_json(v: serde_json::Value) -> Result<Self, String> {
        let mut request = Self::new()?;

        // Iterate over the JSON object and populate the Request
        if let Value::Object(map) = v {
            for (key, value) in map {
                match value {
                    Value::String(s) => {
                        // Treat single strings as a slice of length 1
                        request.set(&key, &[s])?;
                    }
                    Value::Array(arr) => {
                        // Collect string values from the array
                        let values: Vec<String> = arr
                            .into_iter()
                            .filter_map(|val| {
                                if let Value::String(s) = val {
                                    Some(s)
                                } else {
                                    None // You can handle non-string items here if needed
                                }
                            })
                            .collect();
                        request.set(&key, &values)?;
                    }
                    _ => {
                        // Handle other types if necessary
                        return Err(format!("Unsupported value type for key '{}'", key).into());
                    }
                }
            }
        } else {
            return Err("Expected a JSON object at the root".into());
        }
        Ok(request)
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        unsafe {
            (FDBLIB.fdb_delete_request)(self.request);
        }
    }
}
