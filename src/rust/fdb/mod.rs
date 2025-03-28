extern crate libc;
use libc::{c_char, c_int, c_long, size_t};

use std::ffi::CString;

pub mod dataretriever;
pub mod key;
pub mod listiterator;
pub mod request;

use dataretriever::DataRetriever;
use dataretriever::FdbDataReader;
use listiterator::ListIterator;
use request::CRequest;
use request::Request;

use libloading::Library;
use once_cell::sync::Lazy;

mod macros;
use crate::generate_library_wrapper;
use std::sync::Arc;

// FDB C API functions
generate_library_wrapper! {
    FdbApiWrapper {
        fn fdb_new_handle(fdb: *mut *mut FdbHandle) -> c_int;
        fn fdb_initialise() -> c_int;
        fn fdb_new_handle_from_yaml(fdb: *mut *mut FdbHandle, system_config: *const c_char, user_config: *const c_char) -> c_int;
        fn fdb_retrieve(fdb: *mut FdbHandle, req: *mut CRequest, dr: *mut FdbDataReader) -> c_int;
        fn fdb_archive_multiple(fdb: *mut FdbHandle, req: *mut CRequest, data: *const c_char, length: size_t) -> c_int;
        fn fdb_flush(fdb: *mut FdbHandle) -> c_int;
        fn fdb_delete_handle(fdb: *mut FdbHandle);

        // Data reader functions
        fn fdb_new_datareader(dr: *mut *mut FdbDataReader) -> c_int;
        fn fdb_datareader_open(dr: *mut FdbDataReader, size: *mut c_long) -> c_int;
        fn fdb_datareader_close(dr: *mut FdbDataReader) -> c_int;
        fn fdb_datareader_tell(dr: *mut FdbDataReader, pos: *mut c_long) -> c_int;
        fn fdb_datareader_seek(dr: *mut FdbDataReader, pos: c_long) -> c_int;
        // fn fdb_datareader_skip(dr: *mut FdbDataReader, count: c_long) -> c_int;
        fn fdb_datareader_read(dr: *mut FdbDataReader, buf: *mut libc::c_void, count: c_long, read: *mut c_long) -> c_int;
        fn fdb_delete_datareader(dr: *mut FdbDataReader);

        // Key functions
        fn fdb_new_key(key: *mut *mut CKey) -> c_int;
        fn fdb_key_add(key: *mut CKey, param: *const c_char, value: *const c_char) -> c_int;
        fn fdb_delete_key(key: *mut CKey);

        // Request functions
        fn fdb_new_request(request: *mut *mut CRequest) -> c_int;
        fn fdb_request_add(request: *mut CRequest, name: *const c_char, values: *const *const c_char, n_values: libc::size_t) -> c_int;
        fn fdb_delete_request(request: *mut CRequest);

        fn fdb_list(fdb: *mut FdbHandle, req: *mut CRequest, it: *mut *mut FdbListIterator, duplicates : bool) -> c_int;

        // ListIterator functions
        fn fdb_listiterator_next(it: *mut FdbListIterator) -> c_int;
        fn fdb_listiterator_attrs(
            it: *mut FdbListIterator,
            uri: *mut *const c_char,
            off: *mut size_t,
            len: *mut size_t,
        ) -> c_int;
        fn fdb_listiterator_splitkey(it: *mut FdbListIterator, key: *mut FdbSplitKey) -> c_int;
        fn fdb_delete_listiterator(it: *mut FdbListIterator);

        // SplitKey functions, extracts path, len, offset, and request = {key : value} from each key
        fn fdb_new_splitkey(key : *mut *mut FdbSplitKey) -> c_int;
        fn fdb_splitkey_next_metadata(it : *mut FdbSplitKey, key: *mut *const c_char, value: *mut *const c_char, level: *mut size_t) -> c_int;
        fn fdb_delete_splitkey(key : *mut FdbSplitKey);
    }
}

// Define the fdb library as a global, lazily-initialized library
pub static FDBLIB: Lazy<Arc<FdbApiWrapper>> = Lazy::new(|| {
    let libpath = "/Users/math/micromamba/envs/qubed/lib/libfdb5.dylib";
    let raw_lib = Library::new(&libpath).expect("Failed to load library");
    let fdblib_wrapper = FdbApiWrapper::load(raw_lib)
        .map_err(|e| e.to_string())
        .expect("Failed to wrap FDB5 library");
    Arc::new(fdblib_wrapper)
});

#[repr(C)]
pub struct FdbSplitKey {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FdbListIterator {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FdbSplitKeyMetadata {
    _private: [u8; 0],
}

#[repr(C)]
pub struct CKey {
    _empty: [u8; 0],
}

#[repr(C)]
pub struct FdbHandle {
    _empty: [u8; 0],
}

pub struct FDB {
    handle: *mut FdbHandle,
}

impl FDB {
    pub fn new(config: Option<&str>) -> Result<Self, String> {
        let mut handle: *mut FdbHandle = std::ptr::null_mut();

        unsafe {
            let result = (FDBLIB.fdb_initialise)();
            if result != 0 {
                return Err("Failed to initialise FDB".into());
            }
        }

        let result: i32 = match config {
            Some(cfg) => {
                let sys_cfg = CString::new(cfg)
                    .map_err(|_| "System Config contains null byte".to_string())?;
                let usr_cfg =
                    CString::new("").map_err(|_| "User Config contains null byte".to_string())?;
                unsafe {
                    (FDBLIB.fdb_new_handle_from_yaml)(
                        &mut handle,
                        sys_cfg.as_ptr(),
                        usr_cfg.as_ptr(),
                    )
                }
            }
            None => unsafe { (FDBLIB.fdb_new_handle)(&mut handle) },
        };

        if result != 0 {
            return Err("Failed to create FDB handle".into());
        }

        Ok(Self { handle })
    }

    pub fn archive_multiple(&self, request: Option<&Request>, data: &[u8]) -> Result<(), String> {
        let req_ptr = match request {
            Some(req) => req.as_ptr(),
            None => std::ptr::null_mut(),
        };

        let result = unsafe {
            (FDBLIB.fdb_archive_multiple)(
                self.handle,
                req_ptr,
                data.as_ptr() as *const c_char,
                data.len(),
            )
        };
        if result != 0 {
            return Err("Failed to archive data".into());
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<(), String> {
        let result = unsafe { (FDBLIB.fdb_flush)(self.handle) };
        if result != 0 {
            return Err("Failed to flush FDB".into());
        }
        Ok(())
    }

    pub fn retrieve(&self, request: &Request) -> Result<DataRetriever, String> {
        DataRetriever::new(self, request)
    }

    pub fn list(
        &self,
        request: &Request,
        key: bool,
        duplicates: bool,
    ) -> Result<ListIterator, String> {
        ListIterator::new(self, request, key, duplicates)
    }
}

impl Drop for FDB {
    fn drop(&mut self) {
        unsafe {
            (FDBLIB.fdb_delete_handle)(self.handle);
        }
    }
}

// // make a small test
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_fdb_new() {
//         let fdb = FDB::new(None);
//         assert!(fdb.is_ok());
//     }
// }
