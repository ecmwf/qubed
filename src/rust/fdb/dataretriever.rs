use std::io;
use std::io::ErrorKind;
use std::io::SeekFrom;

use super::request::Request;
use super::FDB;
use super::FDBLIB;

#[repr(C)]
pub struct FdbDataReader {
    _empty: [u8; 0],
}

pub struct DataRetriever {
    datareader: *mut FdbDataReader,
    opened: bool,
}

impl DataRetriever {
    pub fn new(fdb: &FDB, request: &Request) -> Result<Self, String> {
        // Create a new data reader
        let mut datareader: *mut FdbDataReader = std::ptr::null_mut();
        let result = unsafe { (FDBLIB.fdb_new_datareader)(&mut datareader) };
        if result != 0 {
            return Err("Failed to create data reader".into());
        }

        // Retrieve data
        let result = unsafe { (FDBLIB.fdb_retrieve)(fdb.handle, request.as_ptr(), datareader) };
        if result != 0 {
            unsafe { (FDBLIB.fdb_delete_datareader)(datareader) };
            return Err("Failed to initiate data retrieval".into());
        }

        Ok(Self {
            datareader,
            opened: false,
        })
    }

    pub fn open(&mut self) -> Result<(), io::Error> {
        if !self.opened {
            let result =
                unsafe { (FDBLIB.fdb_datareader_open)(self.datareader, std::ptr::null_mut()) };
            if result != 0 {
                return Err(io::Error::new(
                    ErrorKind::Other,
                    "Failed to open data reader",
                ));
            }
            self.opened = true;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        if self.opened {
            unsafe { (FDBLIB.fdb_datareader_close)(self.datareader) };
            self.opened = false;
        }
    }

    pub fn tell(&mut self) -> Result<libc::c_long, io::Error> {
        self.open()?;
        let mut pos = 0;
        let result = unsafe { (FDBLIB.fdb_datareader_tell)(self.datareader, &mut pos) };
        if result != 0 {
            return Err(io::Error::new(
                ErrorKind::Other,
                "Failed to tell in data reader",
            ));
        }
        Ok(pos)
    }
}

impl std::io::Seek for DataRetriever {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as libc::c_long,

            SeekFrom::End(_offset) => {
                // Don't know size of stream, so can't seek from end
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Seek from end is not supported for this stream",
                ));
            }

            SeekFrom::Current(offset) => {
                let current_pos = self.tell()? as i64;
                (current_pos + offset) as libc::c_long
            }
        };

        let result = unsafe { (FDBLIB.fdb_datareader_seek)(self.datareader, new_pos) };
        if result != 0 {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to seek in data reader",
            ))
        } else {
            Ok(new_pos as u64)
        }
    }
}
impl std::io::Read for DataRetriever {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.open()?;

        let mut read = 0;
        let result = unsafe {
            (FDBLIB.fdb_datareader_read)(
                self.datareader,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len() as libc::c_long,
                &mut read,
            )
        };

        if result != 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to read from data reader",
            ))
        } else {
            Ok(read as usize)
        }
    }
}

impl Drop for DataRetriever {
    fn drop(&mut self) {
        self.close();
        unsafe {
            (FDBLIB.fdb_delete_datareader)(self.datareader);
        }
    }
}
