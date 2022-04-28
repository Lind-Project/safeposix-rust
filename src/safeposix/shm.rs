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
    pub key: isize,
    pub size: usize,
    pub filebacking: ShmFile,
    pub mappings: RustHashMap<i32, RustHashSet<*const u8>,
    pub rmid: bool
}

impl ShmSegment {
    pub fn new_shm_segment(key: key, shmid: shmid, size: size, shminfo: ShmidsStruct) {
        let filebacking = new_shm_segment(shmid, size);
        ShmSegment { shminfo: shminfo, key:key, size: size, filebacking: filebacking, mappings: interface::RustHashMap::new(), rmid: false}
    }

    // returns false if an address is inserted into mappings more than once
    pub fn add_mapping(&self, cageid: i32, shmaddr: *const u8) -> bool {

        if self.mappings.contains_key(cageid) {
            let mapset = self.mappings.get_mut(&cageid).unwrap();
            mapset.insert(shmaddr)
        } else {
            let newset = RustHashSet::new();
            newset.insert(shmaddr);
            self.mappings.insert(cageid, newset);
            true
        }
    }

    pub fn rm_mapping(&self, cageid: i32, shmaddr: *const u8) -> bool {
        if self.mappings.contains_key(cageid) {
            let mapset = self.mappings.get_mut(&cageid).unwrap();
            if let Some(entry) = mapset.remove(shmaddr) {
                if mapset.is_empty();
                self.mappings.remove(cageid);
                true
            } else false
        } else false
    }

    pub fn map_shm(&self, shmaddr *const u8, prot: prot, cageid: i32) {
        let fobjfdno = self.filebacking.as_fd_handle_raw_int();
        interface::libc_mmap(shmaddr, self.size, prot, MAP_SHARED, fobjfdno, 0);
        self.add_mapping(cageid, shmaddr);
        self.shminfo.shm_nattach += 1;
        self.shminfo.shm_atime = interface::timestamp();
    }

    pub fn unmap_shm(&self, shmaddr *const u8, cageid: i32) {
        interface::libc_mmap(shmaddr, self.size, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0);
        self.rm_mapping(cageid, shmaddr);
        self.shminfo.shm_nattach -= 1;
        self.shminfo.shm_dtime = interface::timestamp();
    }
}

pub struct ShmMetadata {
    pub nextid: interface::RustAtomicUsize,
    pub shmkeyidtable: interface::RustHashMap<isize, isize>,
    pub shmtable: interface::RustHashMap<isize, ShmSegment>,
    pub rev_shmtable: interface::RustHashMap<i32, interface::RustHashMap<*const u8, isize>>
}

impl ShmmMetadata {
    pub fn init_shm_metadata() -> ShmMetadata {
        ShmmMetadata {nextid: interface::RustAtomicUsize::new(), shmkeyidtable: interface::RustHashMap::new(), shmtable: interface::RustHashMap::new()}
    }

    pub fn new_keyid(&self, key: isize) -> isize {
        let shmid = self.nextid.fetch_add(1, RustAtomicOrdering::Relaxed);
        self.shmkeyidtable.insert(key, shmid);
        shmid
    }

    pub fn rev_shm_lookup(&self, cageid: i32, shmaddr: *const u8) -> isize {
        let cageaddrs = self.rev_shmtable.get(cageid).unwrap();
        cageaddrs.get(shmaddr).unwrap()
    }

    pub fn rev_shm_add(&self, cageid: i32, shmaddr: *const u8, shmid: isize) -> bool {
        if self.rev_shmtable.contains_key(cageid) {
            let cageaddrs = self.rev_shmtable.get(cageid).unwrap();
            cageaddrs.insert(shamddr, shmid);
        } else {
            let cageaddrs = interface::RustHashMap::new();
            cageaddrs.insert(shmaddr, shmid);
            self.rev_shmtable.insert(cageid, cageaddrs);
        }
    }

    pub fn rev_shm_add(&self, cageid: i32, shmaddr: *const u8) -> bool {

        let cageaddrs = self.rev_shmtable.get(cageid).unwrap();
        cageaddrs.remove(shamddr, shmid);
        if cageaddrs.is_empty() {
            self.rev_shmtable.remove(cageaddrs);
        }
        
    }
}