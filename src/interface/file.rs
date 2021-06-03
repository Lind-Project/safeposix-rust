// Author: Nicholas Renner
//
// File related interface
#![feature(once_cell)]
use std::lazy::SyncLazy;

use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::fs;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::slice;



static OPEN_FILES: SyncLazy<Arc<Mutex<HashSet>>> = SyncLazy::new(|| Arc::new(Mutex::new(HashSet::new())));

pub fn listfiles() -> <Vec<String>> {
    let paths = fs::read_dir(&Path::new(
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

pub fn removefile(filename: &str) {
    let openfiles = OPEN_FILES.lock().unwrap();

    if openfiles.contains(filename) {
      panic!("FileInUse");
    }

    let path: PathBuf = [SAFEPOSIX_DIR, filename].iter().collect();

    let absolute_filename = fs::canonicalize(&path);

    if !absolute_filename.exists() {
      panic!("FileNotFoundError");
    }

    fs::remove_file(absolute_filename)?;

    drop(openfiles);
    
}

fn assert_is_allowed_filename(filename: &str) {

  let SAFEPOSIX_DIR = ".";
  let MAX_FILENAME_LENGTH = 120;
  let ILLEGAL_FILENAMES : HashSet<&'static str> = [ ".", "..", "" ].iter().cloned().collect();

  if filename.len() > MAX_FILENAME_LENGTH {
    panic!("ArgumentError: Filename exceeds maximum length.")
  }

  if !filename.chars().all(char::is_alphanumeric) {
    panic!("ArgumentError: Filename has disallowed charachters.")
  }

  if ILLEGAL_FILENAMES.contains(filename) {
    panic!("ArgumentError: Illegal filename.")
  }

  if filename.starts_with(".") {
    panic!("ArgumentError: Filename cannot start with a period.")

  }
}

pub fn emulated_open(filename: &str, create: bool) {
  emulated_file::new(filename, create);
}

pub struct emulated_file {
  filename: &str,
  abs_filename: &str,
  fobj: Option<Arc<Mutex<File>>>
  filesize: i32
}

impl emulated_file {

  fn new(filename: &str, create: bool) {
    assert_is_allowed_filename(filename);

    let openfiles = OPEN_FILES.lock().unwrap();

    if openfiles.contains(filename) {
      panic!("FileInUse");
    }

    let path: PathBuf = [SAFEPOSIX_DIR, filename].iter().collect();
    let absolute_filename = fs::canonicalize(&path);

    if !absolute_filename.exists() {
      if !create {
        panic!("Cannot open non-existent file {}", filename);
      }

      let mut f = File::create(filename)?;
      drop(f);    
    }

    let mut f = File::open(filename)?;

    openfiles.insert(filename);
    let filesize = f.stream_len()?;

    drop(openfiles);

    emulated_file {filename: filename, abs_filename: absolute_filename, fobj: Arc::new(Mutex::new(f)), filesize: filesize}

  }

  fn close(&self) {
    let openfiles = OPEN_FILES.lock().unwrap();

    let fobj = self.fobj.lock().unwrap();
    drop(fobj);
    let fobj = None;

    openfiles.remove(self.filename);

    drop(openfiles);

  }

  unsafe fn readat(&self, ptr: *const u8, length: usize, offset: usize) -> isize {
    
    let mut bytes_read = 0;

    if offset < 0 {
      panic!("Negative offset specified.");
    }

    if length < 0 {
      panic!("Negative length specified.");
    }
    
    let buf = unsafe {
      assert!(!ptr.is_null());
      slice::from_raw_parts_mut(ptr, length)
    };

    match self.fobj {
      None => panic!("{} is already closed.", self.filename),
      Some(f) => { 
        let fobj = self.f.lock().unwrap();
        if offset > self.filesize {
          panic!("Seek offset extends past the EOF!");
        }
        fobj.seek(SeekFrom::Start(offset))?;
        bytes_read = fobj.read(buf);
        fobj.sync_data()?;
  
        drop(fobj)

      }
    }

    bytes_read;
  }


  unsafe fn writeat(&self, ptr: *const u8, length: usize, offset: usize) -> isize {

    let mut bytes_written = 0;

    if offset < 0 {
      panic!("Negative offset specified.");
    }

    if length < 0 {
      panic!("Negative length specified.");
    }

    let buf = unsafe {
      assert!(!ptr.is_null());
      slice::from_raw_parts(ptr, length)
    };

    match self.fobj {
      None => panic!("{} is already closed.", self.filename),
      Some(f) => { 
        let fobj = self.f.lock().unwrap();
        if offset > self.filesize {
          panic!("Seek offset extends past the EOF!");
        }
        fobj.seek(SeekFrom::Start(offset))?;
        bytes_written = fobj.write_all(buf);
        fobj.sync_data()?;
  
        drop(fobj)

      }
    }

    if offset + length > self.filesize {
      self.filesize = offset + length;
    }

    bytes_written;
  }

}