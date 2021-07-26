// Author: Nicholas Renner
//
// Pipes for SafePOSIX based on Lock-Free Circular Buffer

#![allow(dead_code)]
use crate::interface;

use std::slice;
use std::sync::{Arc, Mutex};

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};

pub struct EmulatedPipe {
    write_end: Arc<Mutex<SyncSender<u8>>>,
    read_end: Arc<Mutex<Receiver<u8>>>,
    size: usize,
    eof: bool,
}

impl EmulatedPipe {
    pub fn new_with_capacity(size: usize) -> EmulatedPipe {
        let (mut writer, mut reader) = sync_channel::<u8>(size);
        EmulatedPipe { write_end: Arc::new(Mutex::new(writer)), read_end: Arc::new(Mutex::new(reader)), size: size, eof: false}
    }

    pub fn write_to_pipe(&self, ptr: *const u8, length: usize) -> usize {

        let mut bytes_written = 0;

        let buf = unsafe {
            assert!(!ptr.is_null());
            slice::from_raw_parts(ptr, length)
        };

        // println!("{:?}", buf);


        let mut write_end = self.write_end.lock().unwrap();

        // while bytes_written < length {
            
        //     let mut bytes_to_write = 0;

        //     if (length - bytes_written) > self.size {
        //         bytes_to_write = self.size;
        //     } else {
        //         bytes_to_write = length - bytes_written;
        //     }

        //     // println!("{:?}", bytes_written);

        //     let curr_buf = buf.get(bytes_written..(bytes_written + bytes_to_write)).unwrap();
        //     // println!("{:?}", curr_buf);
        //     for byte in curr_buf {
        //         write_end.send(*byte).unwrap();
        //     }
        //     // println!("sent data.");

        //     bytes_written += bytes_to_write;
        // }
        for byte in buf {
            write_end.send(*byte).unwrap();
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

        // // println!("in read");
        // while bytes_read < length {
            
        //     let mut bytes_to_read = 0;

        //     if (length - bytes_read) > self.size {
        //         bytes_to_read = self.size;
        //     } else {
        //         bytes_to_read = length - bytes_read;
        //     }

        //     let curr_buf = buf.get_mut(bytes_read..(bytes_read + bytes_to_read)).unwrap();
        //     // println!("{:?}", curr_buf);
        //     // println!("reading.");

        //     let mut iter = read_end.iter();
        //     for i in 00..bytes_to_read {
        //         curr_buf[i] = iter.next().unwrap();
        //     }

        //     bytes_read += bytes_to_read;
        // }
            let mut iter = read_end.iter();

            for i in 00..length {
                buf[i] = iter.next().unwrap();
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

    use std::io::{Read, Error};
    use std::fs::File;
    use std::time::Instant;
    

    // #[test]
    // pub fn pipetest() {
    //     let q = unsafe{libc::malloc(mem::size_of::<u8>() * 9) as *mut u8};
    //     unsafe{std::ptr::copy_nonoverlapping("fizzbuzz!".as_bytes().as_ptr() , q as *mut u8, 9)};

    //     let mut testpipe = EmulatedPipe::new_with_capacity(256);

    //     testpipe.write_to_pipe(q, 9);

    //     let b = unsafe{libc::malloc(mem::size_of::<u8>() * 9)} as *mut u8;

    //     testpipe.read_from_pipe(b,9);

    //     println!("{:?}", String::from_utf8(unsafe{std::slice::from_raw_parts(b, 9)}.to_vec()).unwrap());


    //     unsafe {
    //     libc::free(q as *mut libc::c_void);
    //     libc::free(b as *mut libc::c_void);
    //     }
    // }

    // #[test]
    // pub fn biggerpipetest() {


    //     {
    //         let testpipe = interface::RustRfc::new(EmulatedPipe::new_with_capacity(256));
    //         let mut mutpipetable = PIPE_TABLE.write().unwrap();
    //         mutpipetable.insert(1, testpipe);
    //     }
        
        
    //     let sender = std::thread::spawn(move || {


    //         let q = unsafe{libc::malloc(mem::size_of::<u8>() * 2048) as *mut u8};
    //         unsafe{std::ptr::copy_nonoverlapping("In the beginning God created the heaven and the earth. And the earth was without form, and void; and darkness was upon the face of the deep. And the Spirit of God moved upon the face of the waters. 
    //         And God said, Let there be light: and there was light. 
    //         And God saw the light, that it was good: and God divided the light from the darkness. 
    //         And God called the light Day, and the darkness he called Night. And the evening and the morning were the first day. 
    //         And God said, Let there be a firmament in the midst of the waters, and let it divide the waters from the waters. 
    //         And God made the firmament, and divided the waters which were under the firmament from the waters which were above the firmament: and it was so. 
    //         And God called the firmament Heaven. And the evening and the morning were the second day. 
    //         And God said, Let the waters under the heaven be gathered together unto one place, and let the dry land appear: and it was so. 
    //         And God called the dry land Earth; and the gathering together of the waters called he Seas: and God saw that it was good. 
    //         And God said, Let the earth bring forth grass, the herb yielding seed, and the fruit tree yielding fruit after his kind, whose seed is in itself, upon the earth: and it was so. 
    //         And the earth brought forth grass, and herb yielding seed after his kind, and the tree yielding fruit, whose seed was in itself, after his kind: and God saw that it was good. 
    //         And the evening and the morning were the third day. 
    //         And God said, Let there be lights in the firmament of the heaven to divide the day from the night; and let them be for signs, and for seasons, and for days, and years: 
    //         And let them be for lights in the firmament of the heaven to give light upon the earth: and it was so. 
    //         And God made two great lights; the greater light to rule the day, and the lesser light to rule the night: he made the stars also. 
    //         And God set them in the firmament of the heaven to give light upon the earth, 
    //         And to rule over the day and over the night, and to divide the light from the darkness: and God s".as_bytes().as_ptr() , q as *mut u8, 2048)};
            
    //         let testpipe = {PIPE_TABLE.read().unwrap().get(&1).unwrap().clone() };

    //         testpipe.write_to_pipe(q, 2048);
    //         unsafe {libc::free(q as *mut libc::c_void); }
    //     });



    //     let receiver = std::thread::spawn(move || {
    //         let b = unsafe{libc::malloc(mem::size_of::<u8>() * 2048)} as *mut u8;

    //         let testpipe = {PIPE_TABLE.read().unwrap().get(&1).unwrap().clone() };
            
    //         testpipe.read_from_pipe(b,2048);
    //         print!("{:?}", String::from_utf8(unsafe{std::slice::from_raw_parts(b, 2048)}.to_vec()).unwrap());
    //         unsafe{libc::free(b as *mut libc::c_void);}
    //     });

    //     sender.join().unwrap();
    //     receiver.join().unwrap();

    // }


    #[test]
    pub fn hugefilepipetest() {
        let bytes_to_read: usize = 131072;
        let num_writes: usize = 8192;
        let now = Instant::now();

        {
            let testpipe = interface::RustRfc::new(EmulatedPipe::new_with_capacity(256));
            let mut mutpipetable = PIPE_TABLE.write().unwrap();
            mutpipetable.insert(1, testpipe);
        }
        
        
        let sender = std::thread::spawn(move || {


            let testpipe = {PIPE_TABLE.read().unwrap().get(&1).unwrap().clone() };

            let mut f = File::open("test1gb.txt").unwrap();
        
            let mut buf = vec![0u8; bytes_to_read];
        
            for _it in 0..num_writes {
                f.read_exact(&mut buf).unwrap();
                testpipe.write_to_pipe(buf.as_mut_ptr(), bytes_to_read);
                // println!("{:?}", _it);
                // println!("{:?}", buf);
            }
        

        });



        let receiver = std::thread::spawn(move || {
            let mut buf: Vec<u8> = Vec::with_capacity(bytes_to_read * num_writes);
            let testpipe = {PIPE_TABLE.read().unwrap().get(&1).unwrap().clone() };
            
            for i in 0..num_writes {
                let bufptr = buf.as_mut_ptr();
                testpipe.read_from_pipe(bufptr, bytes_to_read);
                unsafe{bufptr.add(bytes_to_read);}
                // println!("{:?}", i);
            }

            println!("{:?}", buf);

        });

        sender.join().unwrap();
        receiver.join().unwrap();
        println!("{}", now.elapsed().as_micros());

    }
}
