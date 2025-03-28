use libc::size_t;
use std::ffi::CStr;
use std::os::raw::c_char;

use super::{FdbListIterator, FdbSplitKey};
use super::{Request, FDB, FDBLIB};

// Represents an individual key-value pair like {class : rd}, level = 0
#[derive(Debug, PartialEq, Clone)]
pub struct KeyValueLevel {
    pub key: String,
    pub value: String,
    pub level: usize,
}
#[derive(Debug, PartialEq)]
pub struct ListItem {
    pub uri: String,
    pub offset: usize,
    pub length: usize,
    pub request: Option<Vec<KeyValueLevel>>,
}

pub struct ListIterator {
    handle: *mut FdbListIterator,
    key: bool, // Whether we're extracting keys or just path, len, offset for each list item.
}

impl ListIterator {
    pub fn new(fdb: &FDB, request: &Request, key: bool, duplicates: bool) -> Result<Self, String> {
        let mut it: *mut FdbListIterator = std::ptr::null_mut();

        let result =
            unsafe { (FDBLIB.fdb_list)(fdb.handle, request.as_ptr(), &mut it, duplicates) };
        if result != 0 {
            return Err(format!("fdb_list failed with error code {}", result));
        }

        if it.is_null() {
            return Err("fdb_list returned a null iterator".into());
        }

        Ok(ListIterator {
            handle: it,
            key: key,
        })
    }

    // Extracts the keys and values from the list item
    pub fn get_request_for_key(&self) -> Result<Vec<KeyValueLevel>, String> {
        if !self.key {
            return Err("Getting keys is not enabled for this iterator.".into());
        }

        let mut key_ptr: *mut FdbSplitKey = std::ptr::null_mut();
        let result = unsafe { (FDBLIB.fdb_new_splitkey)(&mut key_ptr) };
        if result != 0 {
            return Err(format!(
                "fdb_new_splitkey failed with error code {}",
                result
            ));
        }

        let result = unsafe { (FDBLIB.fdb_listiterator_splitkey)(self.handle, key_ptr) };
        if result != 0 {
            return Err(format!(
                "fdb_listiterator_splitkey failed with error code {}",
                result
            ));
        }

        if key_ptr.is_null() {
            return Err("fdb_listiterator_splitkey returned a null key".into());
        }

        let mut metadata = Vec::new();

        loop {
            let mut k: *const c_char = std::ptr::null();
            let mut v: *const c_char = std::ptr::null();
            let mut level: size_t = 0;

            let meta_result =
                unsafe { (FDBLIB.fdb_splitkey_next_metadata)(key_ptr, &mut k, &mut v, &mut level) };
            if meta_result != 0 || k.is_null() || v.is_null() {
                break; // No more metadata
            }

            let key = unsafe {
                CStr::from_ptr(k)
                    .to_str()
                    .map_err(|_| "Invalid UTF-8 in splitkey key".to_string())?
                    .to_owned()
            };

            let value = unsafe {
                CStr::from_ptr(v)
                    .to_str()
                    .map_err(|_| "Invalid UTF-8 in splitkey value".to_string())?
                    .to_owned()
            };

            metadata.push(KeyValueLevel {
                key,
                value,
                level: level as usize,
            });
        }

        // Clean up the splitkey instance
        unsafe {
            (FDBLIB.fdb_delete_splitkey)(key_ptr);
        }

        Ok(metadata)
    }
}

impl Iterator for ListIterator {
    type Item = ListItem;

    fn next(&mut self) -> Option<Self::Item> {
        // Advance the iterator
        let result = unsafe { (FDBLIB.fdb_listiterator_next)(self.handle) };
        if result != 0 {
            // Assuming non-zero indicates no more items or an error
            return None;
        }

        // Retrieve attributes
        let mut uri_ptr: *const c_char = std::ptr::null();
        let mut off: size_t = 0;
        let mut len: size_t = 0;

        let attrs_result = unsafe {
            (FDBLIB.fdb_listiterator_attrs)(self.handle, &mut uri_ptr, &mut off, &mut len)
        };
        if attrs_result != 0 || uri_ptr.is_null() {
            // Handle error or end of iteration
            return None;
        }

        // Convert C string to Rust String
        let uri = unsafe {
            CStr::from_ptr(uri_ptr)
                .to_str()
                .unwrap_or("Invalid UTF-8")
                .to_owned()
        };

        // If we're extracting keys, do it.
        let request = if self.key {
            match self.get_request_for_key() {
                Ok(data) => Some(data),
                Err(e) => {
                    eprintln!("Error retrieving splitkey metadata: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Some(ListItem {
            uri,
            offset: off as usize,
            length: len as usize,
            request,
        })
    }
}

impl Drop for ListIterator {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                (FDBLIB.fdb_delete_listiterator)(self.handle);
            }
        }
    }
}
