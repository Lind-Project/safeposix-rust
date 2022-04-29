// Filesystem metadata struct
#![allow(dead_code)]

use crate::interface;
use super::syscalls::fs_constants::*;
use super::syscalls::sys_constants::*;

use super::cage::Cage;

pub static SHM_METADATA: interface::RustLazyGlobal<interface::RustRfc<ShmMetadata>> = 
    interface::RustLazyGlobal::new(|| interface::RustRfc::new(ShmMetadata::init_shm_metadata())); 

pub struct ShmSegment {
    pub shminfo: interface::ShmidsStruct,
    pub key: i32,
    pub size: usize,
    pub filebacking: interface::ShmFile,
    pub rmid: bool
}

pub fn new_shm_segment(key: i32, size: usize, cageid: i32, uid: u32, gid: u32, mode: u16) -> ShmSegment {
    ShmSegment::new(key, size, cageid, uid, gid, mode)
}

impl ShmSegment {
    pub fn new(key: i32, size: usize, cageid: i32, uid: u32, gid: u32, mode: u16) -> ShmSegment {
        let filebacking = interface::new_shm_backing(key, size).unwrap();

        let time = interface::timestamp() as isize; //We do a real timestamp now
        let permstruct = interface::IpcPermStruct { __key: key, uid: uid, gid: gid, cuid: uid, cgid: gid, mode: mode, __seq: 0 };
        let shminfo = interface::ShmidsStruct {shm_perm: permstruct, shm_segsz: size, shm_atime: 0, shm_dtime: 0, shm_ctime: time, shm_cpid: cageid, shm_lpid: 0, shm_nattach: 0};

        ShmSegment { shminfo: shminfo, key:key, size: size, filebacking: filebacking, rmid: false}
    }

    pub fn map_shm(&mut self, shmaddr: *mut u8, prot: i32) {
        let fobjfdno = self.filebacking.as_fd_handle_raw_int();
        interface::libc_mmap(shmaddr, self.size, prot, MAP_SHARED, fobjfdno, 0);
        self.shminfo.shm_nattach += 1;
        self.shminfo.shm_atime = interface::timestamp() as isize;
    }

    pub fn unmap_shm(&mut self, shmaddr: *mut u8) {
        interface::libc_mmap(shmaddr, self.size, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0);
        self.shminfo.shm_nattach -= 1;
        self.shminfo.shm_dtime = interface::timestamp() as isize;
    }
}

pub struct ShmMetadata {
    pub nextid: interface::RustAtomicI32,
    pub shmkeyidtable: interface::RustHashMap<i32, i32>,
    pub shmtable: interface::RustHashMap<i32, ShmSegment>,
    pub rev_shmtable: interface::RustHashMap<i32, interface::RustHashMap<u32, i32>>
}

impl ShmMetadata {
    pub fn init_shm_metadata() -> ShmMetadata {
        ShmMetadata { nextid: interface::RustAtomicI32::new(1), shmkeyidtable: interface::RustHashMap::new(), shmtable: interface::RustHashMap::new(), rev_shmtable: interface::RustHashMap::new() }
    }

    pub fn new_keyid(&self, key: i32) -> i32 {
        let shmid = self.nextid.fetch_add(1, interface::RustAtomicOrdering::Relaxed);
        self.shmkeyidtable.insert(key, shmid);
        shmid
    }

    pub fn rev_shm_lookup(&self, cageid: i32, shmaddr: *mut u8) -> Option<i32> {
        let shmint = shmaddr as u32;
        if !self.rev_shmtable.contains_key(&cageid) { return None; }
        let cageaddrs = self.rev_shmtable.get(&cageid).unwrap();
        if let Some(shmid) = cageaddrs.get(&shmint){ 
            Some(*shmid)
        } else { None }
    }

    pub fn rev_shm_add(&self, cageid: i32, shmaddr: *mut u8, shmid: i32) {
        let shmint = shmaddr as u32;
        if self.rev_shmtable.contains_key(&cageid) {
            let cageaddrs = self.rev_shmtable.get(&cageid).unwrap();
            cageaddrs.insert(shmint, shmid);
        } else {
            let cageaddrs = interface::RustHashMap::new();
            cageaddrs.insert(shmint, shmid);
            self.rev_shmtable.insert(cageid, cageaddrs);
        }
    }

    pub fn rev_shm_rm(&self, cageid: i32, shmaddr: *mut u8)  {
        let shmint = shmaddr as u32;
        let cageaddrs = self.rev_shmtable.get(&cageid).unwrap();
        cageaddrs.remove(&shmint);
        if cageaddrs.is_empty() {
            self.rev_shmtable.remove(&cageid);
        }
    }
}