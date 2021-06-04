// Author: Nicholas Renner
//
// File related interface

use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::fs::{self, File};
use std::env;
use std::path::{PathBuf, Path};
use std::lazy::SyncLazy;

const SAFEPOSIX_DIR: &str = ".";
const MAX_FILENAME_LENGTH: usize = 120;

static OPEN_FILES: SyncLazy<Arc<Mutex<HashSet<String>>>> = SyncLazy::new(|| Arc::new(Mutex::new(HashSet::new())));

pub fn listfiles() -> Vec<String> {
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

pub fn removefile(filename: String) -> std::io::Result<()> {
    let openfiles = OPEN_FILES.lock().unwrap();

    if openfiles.contains(&filename) {
      panic!("FileInUse");
    }

    let path: PathBuf = [SAFEPOSIX_DIR.to_string(), filename].iter().collect();

    let absolute_filename = fs::canonicalize(&path).unwrap();

    if !absolute_filename.exists() {
      panic!("FileNotFoundError");
    }

    fs::remove_file(absolute_filename)?;

    drop(openfiles);
    Ok(())
}

fn assert_is_allowed_filename(filename: &String) {

  if filename.len() > MAX_FILENAME_LENGTH {
    panic!("ArgumentError: Filename exceeds maximum length.")
  }

  if !filename.chars().all(char::is_alphanumeric) {
    panic!("ArgumentError: Filename has disallowed charachters.")
  }

  match filename.as_str() {
    "" | "." | ".." => panic!("ArgumentError: Illegal filename."),
    _ => {}
  }

  if filename.starts_with(".") {
    panic!("ArgumentError: Filename cannot start with a period.")

  }
}

pub fn emulated_open(filename: String, create: bool) -> std::io::Result<EmulatedFile> {
  EmulatedFile::new(filename, create)
}

pub struct EmulatedFile {
  filename: String,
  abs_filename: PathBuf,
  fobj: Arc<Mutex<File>>,
  filesize: usize,
}

impl EmulatedFile {

  fn new(filename: String, create: bool) -> std::io::Result<EmulatedFile> {
    assert_is_allowed_filename(&filename);

    let mut openfiles = OPEN_FILES.lock().unwrap();

    if openfiles.contains(&filename) {
      panic!("FileInUse");
    }

    let path: PathBuf = [SAFEPOSIX_DIR.to_string(), filename.clone()].iter().collect();
    let absolute_filename = fs::canonicalize(&path)?;

    if !absolute_filename.exists() {
      if !create {
        panic!("Cannot open non-existent file {}", filename);
      }

      let f = File::create(filename.clone())?;
      drop(f);    
    }

    let f = File::open(filename.clone())?;

    openfiles.insert(filename.clone());
    f.sync_all()?;
    let filesize = f.metadata()?.len();

    drop(openfiles);

    Ok(EmulatedFile {filename: filename, abs_filename: absolute_filename, fobj: Arc::new(Mutex::new(f)), filesize: filesize as usize})

  }

  fn close(&mut self) {
    let mut openfiles = OPEN_FILES.lock().unwrap();

    let fobj = self.fobj.lock().unwrap();

    drop(fobj);
    openfiles.remove(&self.filename);

    drop(openfiles);

  }

}
