// Author: Nicholas Renner
//
// Pipes for SafePOSIX based on Lock-Free Circular Buffer

#![allow(dead_code)]
use crate::interface;

use std::slice;
use std::sync::{Arc, Mutex};

pub use spsc_bip_buffer::{BipBufferReader, BipBufferWriter, bip_buffer_with_len}; // lock-free circular buffer for pipes


pub struct EmulatedPipe {
    write_end: Arc<Mutex<BipBufferWriter>>,
    read_end: Arc<Mutex<BipBufferReader>>,
    size: usize,
    eof: bool,
}

impl EmulatedPipe {
    pub fn new_with_capacity(size: usize) -> EmulatedPipe {
        let (mut writer, mut reader) = bip_buffer_with_len(size);
        EmulatedPipe { write_end: Arc::new(Mutex::new(writer)), read_end: Arc::new(Mutex::new(reader)), size: size, eof: false}
    }

    pub fn write_to_pipe(&self, ptr: *const u8, length: usize) -> usize {

        let mut bytes_written = 0;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts(ptr, length)
        };

        println!("{:?}", buf);


        let mut write_end = self.write_end.lock().unwrap();

        while bytes_written < length {
            
            let mut bytes_to_write = 0;

            if (length - bytes_written) > self.size {
                bytes_to_write = self.size;
            } else {
                bytes_to_write = length - bytes_written;
            }

            println!("{:?}", bytes_written);

            let curr_buf = buf.get(bytes_written..(bytes_written + bytes_to_write)).unwrap();
            println!("{:?}", curr_buf);
            let mut reservation = write_end.spin_reserve(bytes_to_write);
            reservation.copy_from_slice(curr_buf);
            reservation.send();
            println!("sent data.");

            bytes_written += bytes_to_write;
        }


        length
    }

    pub fn read_from_pipe(&self, ptr: *mut u8, length: usize) -> usize {

        let mut bytes_read = 0;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, length)
        };

        let mut read_end = self.read_end.lock().unwrap();

        println!("in read");
        while bytes_read < length {
            
            let mut bytes_to_read = 0;

            if (length - bytes_read) > self.size {
                bytes_to_read = self.size;
            } else {
                bytes_to_read = length - bytes_read;
            }

            let curr_buf = buf.get_mut(bytes_read..(bytes_read + bytes_to_read)).unwrap();
            println!("{:?}", curr_buf);
            println!("reading.");

            while read_end.valid().len() < bytes_to_read {
                println!("reading");
            }
            println!("{:?}", read_end.valid().len());

            curr_buf.copy_from_slice(read_end.valid());
            read_end.consume(bytes_to_read);

            bytes_read += bytes_to_read;
        }


        length
    }

}


#[cfg(test)]
mod tests {
    extern crate libc;
    use std::mem;
    use std::thread;
    use super::*;
    use crate::safeposix::filesystem::PIPE_TABLE;


    // #[test]
    // pub fn pipetest() {
    //     let q = unsafe{libc::malloc(mem::size_of::<u8>() * 9) as *mut u8};
    //     unsafe{std::ptr::copy_nonoverlapping("fizzbuzz!".as_bytes().as_ptr() , q as *mut u8, 9)};

    //     let mut testpipe = EmulatedPipe::new_with_capacity(256);

    //     testpipe.write_to_pipe(q, 9);

    //     let b = unsafe{libc::malloc(mem::size_of::<u8>() * 9)} as *mut u8;

    //     testpipe.read_from_pipe(b,9);

    //     // println!("{:?}", String::from_utf8(unsafe{std::slice::from_raw_parts(b, 9)}.to_vec()).unwrap());


    //     unsafe {
    //     libc::free(q as *mut libc::c_void);
    //     libc::free(b as *mut libc::c_void);
    //     }
    // }

    #[test]
    pub fn biggerpipetest() {


        {
            let testpipe = interface::RustRfc::new(EmulatedPipe::new_with_capacity(256));
            let mut mutpipetable = PIPE_TABLE.write().unwrap();
            mutpipetable.insert(1, testpipe);
        }
        
        
        let sender = std::thread::spawn(move || {

            println!("starting write!");

            let q = unsafe{libc::malloc(mem::size_of::<u8>() * 2048) as *mut u8};
            unsafe{std::ptr::copy_nonoverlapping("In the beginning God created the heaven and the earth. And the earth was without form, and void; and darkness was upon the face of the deep. And the Spirit of God moved upon the face of the waters. 
            And God said, Let there be light: and there was light. 
            And God saw the light, that it was good: and God divided the light from the darkness. 
            And God called the light Day, and the darkness he called Night. And the evening and the morning were the first day. 
            And God said, Let there be a firmament in the midst of the waters, and let it divide the waters from the waters. 
            And God made the firmament, and divided the waters which were under the firmament from the waters which were above the firmament: and it was so. 
            And God called the firmament Heaven. And the evening and the morning were the second day. 
            And God said, Let the waters under the heaven be gathered together unto one place, and let the dry land appear: and it was so. 
            And God called the dry land Earth; and the gathering together of the waters called he Seas: and God saw that it was good. 
            And God said, Let the earth bring forth grass, the herb yielding seed, and the fruit tree yielding fruit after his kind, whose seed is in itself, upon the earth: and it was so. 
            And the earth brought forth grass, and herb yielding seed after his kind, and the tree yielding fruit, whose seed was in itself, after his kind: and God saw that it was good. 
            And the evening and the morning were the third day. 
            And God said, Let there be lights in the firmament of the heaven to divide the day from the night; and let them be for signs, and for seasons, and for days, and years: 
            And let them be for lights in the firmament of the heaven to give light upon the earth: and it was so. 
            And God made two great lights; the greater light to rule the day, and the lesser light to rule the night: he made the stars also. 
            And God set them in the firmament of the heaven to give light upon the earth, 
            And to rule over the day and over the night, and to divide the light from the darkness: and God s".as_bytes().as_ptr() , q as *mut u8, 2048)};
            
            let testpipe = {PIPE_TABLE.read().unwrap().get(&1).unwrap().clone() };

            testpipe.write_to_pipe(q, 2048);
            unsafe {libc::free(q as *mut libc::c_void); }
        });



        let receiver = std::thread::spawn(move || {
            let b = unsafe{libc::malloc(mem::size_of::<u8>() * 2048)} as *mut u8;

            let testpipe = {PIPE_TABLE.read().unwrap().get(&1).unwrap().clone() };
            
            testpipe.read_from_pipe(b,2048);
            println!("{:?}", String::from_utf8(unsafe{std::slice::from_raw_parts(b, 2048)}.to_vec()));
            unsafe{libc::free(b as *mut libc::c_void);}
        });

        sender.join().unwrap();
        receiver.join().unwrap();

    }
}
