// Author: Nicholas Renner
//
// File related interface
#![allow(dead_code)]

use parking_lot::Mutex;
use std::sync::{Arc};
use dashmap::DashSet;
use std::fs::{self, File, OpenOptions, canonicalize};
use std::env;
use std::slice;
pub use std::path::{PathBuf as RustPathBuf, Path as RustPath, Component as RustPathComponent};
pub use std::ffi::CStr as RustCStr;
use std::io::{SeekFrom, Seek, Read, Write};
pub use std::sync::{LazyLock as RustLazyGlobal};

use std::os::unix::io::{AsRawFd, RawFd};
use libc::{mmap, mremap, munmap, PROT_READ, PROT_WRITE, MAP_SHARED, MREMAP_MAYMOVE, off64_t};
use std::ffi::c_void;
use std::convert::TryInto;
use crate::interface::errnos::{Errno, syscall_error};

static OPEN_FILES: RustLazyGlobal<Arc<DashSet<String>>> = RustLazyGlobal::new(|| Arc::new(DashSet::new()));

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
    let openfiles = &OPEN_FILES;

    if openfiles.contains(&filename) {
        panic!("FileInUse");
    }

    let path: RustPathBuf = [".".to_string(), filename].iter().collect();

    let absolute_filename = canonicalize(&path)?; //will return an error if the file does not exist

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

        if OPEN_FILES.contains(&filename) {
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

        OPEN_FILES.insert(filename.clone());
        let filesize = f.metadata()?.len();

        Ok(EmulatedFile {filename: filename, abs_filename: absolute_filename, fobj: Some(Arc::new(Mutex::new(f))), filesize: filesize as usize})

    }

    pub fn close(&self) -> std::io::Result<()> {
        OPEN_FILES.remove(&self.filename);
        Ok(())
    }

    pub fn shrink(&mut self, length: usize) -> std::io::Result<()> {

        if length > self.filesize { 
            panic!("Something is wrong. {} is already smaller than length.", self.filename);
        }
        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => { 
                let fobj = f.lock();
                fobj.set_len(length as u64)?;
                self.filesize = length;         
                Ok(())
            }
        }
    }

     pub fn fdatasync(&self) -> std::io::Result<()> {
        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => {
                let fobj = f.lock();
                fobj.sync_data()?;
                Ok(())
            }
        }
    }

    pub fn fsync(&self) -> std::io::Result<()> {
        match &self.fobj {
            None => panic!("{} is already closed.", self.filename),
            Some(f) => {
                let fobj = f.lock();
                fobj.sync_all()?;
                Ok(())
            }
        }
    }

    pub fn sync_file_range(&self, offset: isize, nbytes: isize, flags: u32) -> i32 {
        let fd = &self.as_fd_handle_raw_int();
        let valid_flags = libc::SYNC_FILE_RANGE_WAIT_BEFORE | libc::SYNC_FILE_RANGE_WRITE | libc::SYNC_FILE_RANGE_WAIT_AFTER;
        if !(flags & !valid_flags == 0){
            return syscall_error(Errno::EINVAL, "sync_file_range", "flags specifies an invalid bit");
        }
        unsafe { libc::sync_file_range(*fd, offset as off64_t, nbytes as off64_t, flags) }
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
                let mut fobj = f.lock();
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
                let mut fobj = f.lock();
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
                let mut fobj = f.lock();
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
                let mut fobj = f.lock();
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
                let mut fobj = f.lock();
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
            wrapped_barefile.lock().as_raw_fd() as i32
        } else {
            -1
        }
    }
}

pub const COUNTMAPSIZE : usize = 8;
pub const MAP_1MB : usize = usize::pow(2, 20);

#[derive(Debug)]
pub struct EmulatedFileMap {
    filename: String,
    abs_filename: RustPathBuf,
    fobj: Arc<Mutex<File>>,
    map: Arc<Mutex<Option<Vec<u8>>>>,
    count: usize,
    countmap:  Arc<Mutex<Option<Vec<u8>>>>,
    mapsize: usize
}

pub fn mapfilenew(filename: String) -> std::io::Result<EmulatedFileMap> {
    EmulatedFileMap::new(filename)
}

impl EmulatedFileMap {

    fn new(filename: String) -> std::io::Result<EmulatedFileMap> {
        // create new file like a normal emulated file, but always create
        assert_is_allowed_filename(&filename);

        let openfiles = &OPEN_FILES;

        if openfiles.contains(&filename) {
            panic!("FileInUse");
        }

        let path: RustPathBuf = [".".to_string(), filename.clone()].iter().collect();
        let f = OpenOptions::new().read(true).write(true).create(true).open(filename.clone()).unwrap();
        let absolute_filename = canonicalize(&path)?;
        openfiles.insert(filename.clone());

        let mapsize = MAP_1MB - COUNTMAPSIZE;   
        // set the file equal to where were mapping the count and the actual map
        let _newsize = f.set_len((COUNTMAPSIZE + mapsize) as u64).unwrap();

        let map : Vec::<u8>;
        let countmap : Vec::<u8>;

        // here were going to map the first 8 bytes of the file as the "count" (amount of bytes written), and then map another 1MB for logging
        unsafe {
            let map_addr = mmap(0 as *mut c_void, MAP_1MB, PROT_READ | PROT_WRITE, MAP_SHARED, f.as_raw_fd() as i32, 0 as i64);
            countmap =  Vec::<u8>::from_raw_parts(map_addr as *mut u8, COUNTMAPSIZE, COUNTMAPSIZE);
            let map_ptr = map_addr as *mut u8;
            map =  Vec::<u8>::from_raw_parts(map_ptr.offset(COUNTMAPSIZE as isize), mapsize, mapsize);
        }
        
        Ok(EmulatedFileMap {filename: filename, abs_filename: absolute_filename, fobj: Arc::new(Mutex::new(f)), map: Arc::new(Mutex::new(Some(map))), count: 0, countmap: Arc::new(Mutex::new(Some(countmap))), mapsize: mapsize})

    }

    pub fn write_to_map(&mut self, bytes_to_write: &[u8]) -> std::io::Result<()> {

        let writelen = bytes_to_write.len();
        
        // if we're writing past the current map, increase the map another 1MB
        if writelen + self.count > self.mapsize {
            self.extend_map();
        }

        let mut mapopt = self.map.lock();
        let map = mapopt.as_deref_mut().unwrap();

        let mapslice = &mut map[self.count..(self.count + writelen)];
        mapslice.copy_from_slice(bytes_to_write);
        self.count += writelen;
    

        // update the bytes written in the map portion
        let mut countmapopt = self.countmap.lock();
        let countmap = countmapopt.as_deref_mut().unwrap();
        countmap.copy_from_slice(&self.count.to_be_bytes());

        Ok(())
    }

    fn extend_map(&mut self) {

        // open count and map to resize mmap, and file to increase file size
        let mut mapopt = self.map.lock();
        let map = mapopt.take().unwrap();
        let mut countmapopt = self.countmap.lock();
        let countmap = countmapopt.take().unwrap();
        let f = self.fobj.lock();

        // add another 1MB to mapsize
        let new_mapsize = self.mapsize + MAP_1MB;
        let _newsize = f.set_len((COUNTMAPSIZE + new_mapsize) as u64).unwrap();

        let newmap : Vec::<u8>;
        let newcountmap : Vec::<u8>;

        // destruct count and map and re-map
        unsafe {
            let (old_count_map_addr, countlen, _countcap) = countmap.into_raw_parts();
            assert_eq!(COUNTMAPSIZE, countlen);
            let (_old_map_addr, len, _cap) = map.into_raw_parts();
            assert_eq!(self.mapsize, len);
            let map_addr = mremap(old_count_map_addr as *mut c_void, COUNTMAPSIZE + self.mapsize, COUNTMAPSIZE + new_mapsize, MREMAP_MAYMOVE);

            newcountmap =  Vec::<u8>::from_raw_parts(map_addr as *mut u8, COUNTMAPSIZE, COUNTMAPSIZE);
            let map_ptr = map_addr as *mut u8;
            newmap =  Vec::<u8>::from_raw_parts(map_ptr.offset(COUNTMAPSIZE as isize), new_mapsize, new_mapsize);
        }

        // replace maps
        mapopt.replace(newmap);
        countmapopt.replace(newcountmap);
        self.mapsize = new_mapsize;
    }

    pub fn close(&self) -> std::io::Result<()> {
        // remove file as open file and deconstruct map
        let openfiles = &OPEN_FILES;
        openfiles.remove(&self.filename);

        let mut mapopt = self.map.lock();
        let map = mapopt.take().unwrap();
        let mut countmapopt = self.countmap.lock();
        let countmap = countmapopt.take().unwrap();

        unsafe {

            let (countmap_addr, countlen, _countcap) = countmap.into_raw_parts();
            assert_eq!(COUNTMAPSIZE, countlen);
            munmap(countmap_addr as *mut c_void, COUNTMAPSIZE);

            let (map_addr, len, _cap) = map.into_raw_parts();
            assert_eq!(self.mapsize, len);
            munmap(map_addr as *mut c_void, self.mapsize);
        }
    
        Ok(())
    }
}

#[derive(Debug)]
pub struct ShmFile {
    fobj: Arc<Mutex<File>>,
    key: i32,
    size: usize
}

pub fn new_shm_backing(key: i32, size: usize) -> std::io::Result<ShmFile> {
    ShmFile::new(key, size)
}

// Mimic shared memory in Linux by creating a file backing and truncating it to the segment size
// We can then safely unlink the file while still holding a descriptor to that segment,
// which we can use to map shared across cages.
impl ShmFile {
    fn new(key: i32, size: usize) -> std::io::Result<ShmFile> {

        // open file "shm-#id"
        let filename = format!("{}{}", "shm-", key);
        let f = OpenOptions::new().read(true).write(true).create(true).open(filename.clone()).unwrap();
        // truncate file to size
        f.set_len(size as u64)?;
        // unlink file
        fs::remove_file(filename)?;
        let shmfile = ShmFile {fobj: Arc::new(Mutex::new(f)), key: key, size: size};

        Ok(shmfile)
    }

    //gets the raw fd handle (integer) from a rust fileobject
    pub fn as_fd_handle_raw_int(&self) -> i32 {
        self.fobj.lock().as_raw_fd() as i32
    }
}

// convert a series of big endian bytes to a size
pub fn convert_bytes_to_size(bytes_to_write: &[u8]) -> usize {
    let sizearray : [u8; 8] = bytes_to_write.try_into().unwrap();
    usize::from_be_bytes(sizearray)
}

#[cfg(test)]
mod tests {
    extern crate libc;
    use std::mem;
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    use std::env;

    #[test]
    pub fn filewritetest() {
      println!("{:?}", listfiles());
      let mut f = openfile("foobar".to_string(), true).expect("?!");
      println!("{:?}", listfiles());
      let q = unsafe{libc::malloc(mem::size_of::<u8>() * 9) as *mut u8};
      unsafe{std::ptr::copy_nonoverlapping("fizzbuzz!".as_bytes().as_ptr() , q as *mut u8, 9)};
      println!("{:?}", f.writeat(q, 9, 0));
      println!("fsync: {:?}", f.fsync().unwrap());
      println!("fdatasync: {:?}", f.fdatasync().unwrap());
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

    #[test]
    fn test_listfiles() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Change the current directory to our temp directory
        env::set_current_dir(&temp_path).unwrap();

        // Create some files
        let file_names = vec!["test1.txt", "test2.txt", "test3.txt"];
        for file_name in &file_names {
            let file_path = temp_path.join(file_name);
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "Testing").unwrap(); 
        }

        // Run the function under test
        let listed_files = listfiles();

        // Check if the listed files match the created files
        let mut listed_files_sorted = listed_files.clone();
        listed_files_sorted.sort();
        let mut expected_files_sorted = file_names.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        expected_files_sorted.sort();

        assert_eq!(listed_files_sorted, expected_files_sorted, "Files listed do not match files created");

        // Restore the original directory
        env::set_current_dir(&original_dir).unwrap();

        // Clean up is handled by the tempfile crate when temp_dir goes out of scope
    }
}
