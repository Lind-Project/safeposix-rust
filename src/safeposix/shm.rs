// Filesystem metadata struct
#![allow(dead_code)]

use crate::interface;
use super::syscalls::fs_constants::*;
use super::syscalls::sys_constants::*;

use super::cage::Cage;

pub static SHM_METADATA: interface::RustLazyGlobal<interface::RustRfc<ShmmMetadata>> = 
    interface::RustLazyGlobal::new(|| interface::RustRfc::new(ShmMetadata::init_shm_metadata())); 

pub struct ShmSegment {
    pub shminfo: ShmidsStruct,
    pub size: size,
    pub filebacking: ShmFile,
    pub mappings: RustHashMap<i32, Vec<*const u8>>,
    pub rmid: bool
}

impl ShmSegment {
    pub fn new_shm_segment(shmid: shmid, size: size, shminfo: ShmidsStruct) {
        let filebacking = new_shm_segment(shmid, size);
        ShmSegment { shminfo: shminfo, size: size, filebacking: filebacking, mappings: interface::RustHashMap::new(), rmid: false}
    }

    pub fn add_mapping() -> {

    }

    pub fn rm_mapping() -> {

    }

    pub fn check_mapping() -> {
        
    }
}

pub struct ShmMetadata {
    pub nextid: interface::RustAtomicUsize,
    pub shmkeyidtable: interface::RustHashMap<usize, usize>
    pub shmtable: interface::RustHashMap<usize, ShmSegment>
}

impl ShmmMetadata {
    pub fn init_shm_metadata() -> ShmMetadata {
        ShmmMetadata {nextid: interface::RustAtomicUsize::new(), shmkeyidtable: interface::RustHashMap::new(), shmtable: interface::RustHashMap::new()}
    }
}