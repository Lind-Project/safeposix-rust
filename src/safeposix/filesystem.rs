// Filesystem metadata struct

use crate::interface;

pub struct FilesystemMetadata {
    nextinode: usize,
    dev_id: usize,
    inodetable: RustHashMap<usize, 
    fileobjecttable: RustHashMap<usize, EmulatedFile>
} 

impl FilesystemMetadata {

}

