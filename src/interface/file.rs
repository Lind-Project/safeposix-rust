// Author: Nicholas Renner
//
// File related interface
#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::env;
use std::slice;
pub use std::path::{PathBuf as RustPathBuf, Path as RustPath, Component as RustPathComponent};
pub use std::ffi::CStr as RustCStr;
use std::io::{SeekFrom, Seek, Read, Write};
use std::io::{self, BufReader};
use std::io::prelude::*;
pub use std::lazy::{SyncLazy as RustLazyGlobal, SyncOnceCell as RustOnceCell};
use std::ops::Deref;

use std::os::unix::io::{AsRawFd, RawFd};
use libc::{mmap, munmap, PROT_READ, PROT_WRITE, MAP_SHARED};
use std::ffi::c_void;

static OPEN_FILES: RustLazyGlobal<Arc<Mutex<HashSet<String>>>> = RustLazyGlobal::new(|| Arc::new(Mutex::new(HashSet::new())));

pub fn listfiles() -> Vec<String> {
    let paths = fs::read_dir(&RustPath::new(
        &env::current_dir().unwrap())).unwrap();
      
    let names =
    paths.filter_map(|entry| {
      entry.ok().and_then(|e|
        e.path().file_name()
        .and_then(|n| n.to_str().map(|s| String::from(s)))
      )
    }).collect::<Vec<String>>();

    return names;
}

pub fn removefile(filename: String) -> std::io::Result<()> {
    let openfiles = OPEN_FILES.lock().unwrap();

    if openfiles.contains(&filename) {
        panic!("FileInUse");
    }

    let path: RustPathBuf = [".".to_string(), filename].iter().collect();

    let absolute_filename = fs::canonicalize(&path)?; //will return an error if the file does not exist

    fs::remove_file(absolute_filename)?;

    Ok(())
}

fn is_allowed_char(c: char) -> bool{
    char::is_alphanumeric(c) || c == '.'
}

// Checker for illegal filenames
fn assert_is_allowed_filename(filename: &String) {

    const MAX_FILENAME_LENGTH: usize = 120;

    if filename.len() > MAX_FILENAME_LENGTH {
        panic!("ArgumentError: Filename exceeds maximum length.")
    }

    if !filename.chars().all(is_allowed_char) {
        println!("'{}'", filename);
        panic!("ArgumentError: Filename has disallowed characters.")
    }

    match filename.as_str() {
        "" | "." | ".." => panic!("ArgumentError: Illegal filename."),
        _ => {}
    }

    if filename.starts_with(".") {
        panic!("ArgumentError: Filename cannot start with a period.")
    }
}

pub fn openfile(filename: String, create: bool) -> std::io::Result<EmulatedFile> {
    EmulatedFile::new(filename, create)
}

#[derive(Debug)]
pub struct EmulatedFile {
    filename: String,
    abs_filename: RustPathBuf,
    fobj: Option<Arc<Mutex<File>>>,
    filesize: usize,
}

pub fn pathexists(filename: String) -> bool {
    assert_is_allowed_filename(&filename);
    let path: RustPathBuf = [".".to_string(), filename.clone()].iter().collect();
    path.exists()
}

impl EmulatedFile {

    fn new(filename: String, create: bool) -> std::io::Result<EmulatedFile> {
        assert_is_allowed_filename(&filename);

        let mut openfiles = OPEN_FILES.lock().unwrap();

        if openfiles.contains(&filename) {
            panic!("FileInUse");
        }

        let path: RustPathBuf = [".".to_string(), filename.clone()].iter().collect();

        let f = if !path.exists() {
            if !create {
              panic!("Cannot open non-existent file {}", filename);
            }

            OpenOptions::new().read(true).write(true).create(true).open(filename.clone())
        } else {
            OpenOptions::new().read(true).write(true).open(filename.clone())
        }?;

        let absolute_filename = fs::canonicalize(&path)?;

        openfiles.insert(filename.clone());
        let filesize = f.metadata()?.len();

        Ok(EmulatedFile {filename: filename, abs_filename: absolute_filename, fobj: Some(Arc::new(Mutex::new(f))), filesize: filesize as usize})

    }

    pub fn close(&self) -> std::io::Result<()> {
        let mut openfiles = OPEN_FILES.lock().unwrap();

        openfiles.remove(&self.filename);
        Ok(())
    }

    pub fn shrink(&mut self, length: usize) -> std::io::Result<()> {

        if length > self.filesize { 
            panic!("Something is wrong. {} is already smaller than length.", self.filename);
        }
        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => { 
                let fobj = f.lock().unwrap();
                fobj.set_len(length as u64)?;
                self.filesize = length;         
                Ok(())
            }
        }
    }

    // Read from file into provided C-buffer
    pub fn readat(&self, ptr: *mut u8, length: usize, offset: usize) -> std::io::Result<usize> {
        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, length)
        };

        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => { 
                let mut fobj = f.lock().unwrap();
                if offset > self.filesize {
                  panic!("Seek offset extends past the EOF!");
                }
                fobj.seek(SeekFrom::Start(offset as u64))?;
                let bytes_read = fobj.read(buf)?;
                Ok(bytes_read)
            }
        }
    }

    // Write to file from provided C-buffer
    pub fn writeat(&mut self, ptr: *const u8, length: usize, offset: usize) -> std::io::Result<usize> {

        let bytes_written;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts(ptr, length)
        };

        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => { 
                let mut fobj = f.lock().unwrap();
                if offset > self.filesize {
                    panic!("Seek offset extends past the EOF!");
                }
                fobj.seek(SeekFrom::Start(offset as u64))?;
                bytes_written = fobj.write(buf)?;
            }
        }

        if offset + length > self.filesize {
            self.filesize = offset + length;
        }

        Ok(bytes_written)
    }

    // Reads entire file into bytes
    pub fn readfile_to_new_bytes(&self) -> std::io::Result<Vec<u8>> {

        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => { 
                let mut stringbuf = Vec::new();
                let mut fobj = f.lock().unwrap();
                fobj.read_to_end(&mut stringbuf)?;
                Ok(stringbuf) // return new buf string
            }
        }
    }

    // Write to entire file from provided bytes
    pub fn writefile_from_bytes(&mut self, buf: &[u8]) -> std::io::Result<()> {

        let length = buf.len();
        let offset = self.filesize;
    
        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => { 
                let mut fobj = f.lock().unwrap();
                if offset > self.filesize {
                    panic!("Seek offset extends past the EOF!");
                }
                fobj.seek(SeekFrom::Start(offset as u64))?;
                fobj.write(buf)?;
            }
        }

        if offset + length > self.filesize {
            self.filesize = offset + length;
        }

        Ok(())
    }

    pub fn zerofill_at(&mut self, offset: usize, count: usize) -> std::io::Result<usize> {
        let bytes_written;
        let buf = vec![0; count];

        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => { 
                let mut fobj = f.lock().unwrap();
                if offset > self.filesize {
                    panic!("Seek offset extends past the EOF!");
                }
                fobj.seek(SeekFrom::Start(offset as u64))?;
                bytes_written = fobj.write(buf.as_slice())?;
            }
        }

        if offset + count > self.filesize {
            self.filesize = offset + count;
        }

        Ok(bytes_written)
    }
    
    //gets the raw fd handle (integer) from a rust fileobject
    pub fn as_fd_handle_raw_int(&self) -> i32 {
        if let Some(wrapped_barefile) = &self.fobj {
            wrapped_barefile.lock().unwrap().as_raw_fd() as i32
        } else {
            -1
        }
    }
}

#[derive(Debug)]
pub struct EmulatedFileMap {
    filename: String,
    abs_filename: RustPathBuf,
    fobj: Arc<Mutex<File>>,
    maps: Arc<Mutex<Vec<Vec<u8>>>>,
    mapptr: usize,
    mapsize: usize
}

pub fn mapfilenew(filename: String) -> std::io::Result<EmulatedFileMap> {
    EmulatedFileMap::new(filename)
}

impl EmulatedFileMap {

    fn new(filename: String) -> std::io::Result<EmulatedFileMap> {
        assert_is_allowed_filename(&filename);

        let mut openfiles = OPEN_FILES.lock().unwrap();

        if openfiles.contains(&filename) {
            panic!("FileInUse");
        }

        let path: RustPathBuf = [".".to_string(), filename.clone()].iter().collect();


        let mut f = OpenOptions::new().read(true).write(true).create(true).open(filename.clone()).unwrap();

        let absolute_filename = fs::canonicalize(&path)?;

        openfiles.insert(filename.clone());

        let mut maps : Vec<Vec<u8>> = Vec::new();

        let mapsize = usize::pow(2, 20);

        let offset: i64 = 0;

        let map_addr = unsafe{mmap(0 as *mut c_void, mapsize, PROT_READ | PROT_WRITE, MAP_SHARED, f.as_raw_fd() as i32, offset)};
        let mmap = unsafe { Vec::<u8>::from_raw_parts(map_addr as *mut u8, mapsize, mapsize) };
      
        maps.push(mmap);
        

        Ok(EmulatedFileMap {filename: filename, abs_filename: absolute_filename, fobj: Arc::new(Mutex::new(f)), maps: Arc::new(Mutex::new(maps)), mapptr: 0, mapsize: mapsize})

    }

    pub fn write_to_map(&mut self, bytes_to_write: &[u8]) -> std::io::Result<()> {

        let mut maps = self.maps.lock().unwrap();
        let writelen = bytes_to_write.len();

        if writelen + self.mapptr < self.mapsize {

            let mapslice = &maps.last().unwrap()[self.mapptr..(self.mapptr + writelen)];
            mapslice.copy_from_slice(bytes_to_write);
            self.mapptr += writelen;
       
        }
        else {

            let firstwrite = self.mapsize - self.mapptr;
            let secondwrite = writelen - firstwrite;
            let mapslice = &maps.last().unwrap()[self.mapptr..(self.mapptr + firstwrite)];
            mapslice.copy_from_slice(&bytes_to_write[0..firstwrite]);
            self.mapptr += firstwrite;

            drop(maps);
            self.increase_map();

            let mut maps = self.maps.lock().unwrap();
            let mapslice = &maps.last().unwrap()[self.mapptr..(self.mapptr + secondwrite)];
            mapslice.copy_from_slice(&bytes_to_write[firstwrite..secondwrite]);
            self.mapptr += secondwrite;

        }

        Ok(())

    }

    fn increase_map(&mut self) {

        let mut maps = self.maps.lock().unwrap();
        let mut f = self.fobj.lock().unwrap();


        let offset = (self.mapsize * maps.len()) as i64;

        let map_addr = unsafe{mmap(0 as *mut c_void, self.mapsize, PROT_READ | PROT_WRITE, MAP_SHARED, f.as_raw_fd() as i32, offset)};
        let mmap = unsafe { Vec::<u8>::from_raw_parts(map_addr as *mut u8, self.mapsize, self.mapsize) };
      
        maps.push(mmap);
        self.mapptr = 0;

    }

    pub fn close(&self) -> std::io::Result<()> {
        let mut openfiles = OPEN_FILES.lock().unwrap();
        openfiles.remove(&self.filename);

        let mut maps = self.maps.lock().unwrap();

        for map in maps.drain(..) {
            munmap(map.as_mut_ptr() as *mut c_void, self.mapsize);
        }

        Ok(())
    }
}



#[cfg(test)]
mod tests {
    extern crate libc;
    use std::mem;
    use super::*;
    #[test]
    pub fn filewritetest() {
      println!("{:?}", listfiles());
      let mut f = openfile("foobar".to_string(), true).expect("?!");
      println!("{:?}", listfiles());
      let q = unsafe{libc::malloc(mem::size_of::<u8>() * 9) as *mut u8};
      unsafe{std::ptr::copy_nonoverlapping("fizzbuzz!".as_bytes().as_ptr() , q as *mut u8, 9)};
      println!("{:?}", f.writeat(q, 9, 0));
      let b = unsafe{libc::malloc(mem::size_of::<u8>() * 9)} as *mut u8;
      println!("{:?}", String::from_utf8(unsafe{std::slice::from_raw_parts(b, 9)}.to_vec()));
      println!("{:?}", f.readat(b, 9, 0));
      println!("{:?}", String::from_utf8(unsafe{std::slice::from_raw_parts(b, 9)}.to_vec()));
      println!("{:?}", f.close());
      unsafe {
        libc::free(q as *mut libc::c_void);
        libc::free(b as *mut libc::c_void);
      }
      println!("{:?}", removefile("foobar".to_string()));
    }
}
