use alloc::alloc::{alloc as rust_alloc, dealloc as rust_dealloc, Layout};
use core::ptr::{self, null_mut};
use crate::{inb, outb};

pub fn serial_init() {
    unsafe{
        outb(0x3F9, 0x00);
        outb(0x3FB, 0x80);
        outb(0x3F8, 0x03);
        outb(0x3F9, 0x00);
        outb(0x3fB, 0x03);
        outb(0x3FA, 0xC7);
        outb(0x3FC, 0x0B);
    }
}

fn serial_write(bin: u8) {
    unsafe {
        while inb(0x3FD) & 0x20 == 0{
            //Yay another stalling thing
            //Waiting for the memory to be safe to write too
            //Dont want any pesky seg faults
        }

        outb(0x3F8, bin);
    }
}

unsafe fn serial_puts(mut s: *const u8) {
    unsafe {
        while *s != 0 {
            serial_write(*s);
            s = s.add(1);
        }
    }
}

const HDR: usize = 16;

fn layout(total: usize) -> Layout {
    unsafe {
        return Layout::from_size_align_unchecked(total, 16);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: usize) -> *mut u8 {
    let check_sums: Option<usize> = size.checked_add(HDR);
    let total = match check_sums {
        Some(t) => t,
        None => return null_mut(),
    };

    unsafe {
        let base = rust_alloc(layout(total));
        if base.is_null() {
            return null_mut();
        }
        *(base as *mut usize) = total;
        return base.add(HDR);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(p: *mut u8) {
    if p.is_null() {
        return;
    }

    unsafe {
        let base = p.sub(HDR);
        let total = *(base as *mut usize);
        rust_dealloc(base, layout(total));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(n: usize, size: usize) -> *mut u8 {
    //Virtually the same as malloc just a. with a c and b. initialises the memory
    let check_sums: Option<usize> = size.checked_mul(HDR);
    let total = match check_sums {
        Some(t) => t,
        None => return null_mut(),
    };

    unsafe {
        let p = malloc(total);
        if !p.is_null() {
            ptr::write_bytes(p, 0, total);
        }
        return p;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(p: *mut u8, size: usize) -> *mut u8 {
    unsafe {
        if p.is_null() {
            return malloc(size);
        }
        if size == 0 {
            free(p);
            return null_mut();
        }
        let old = *(p.sub(HDR) as *mut usize) - HDR;
        let new = malloc(size);
        if new.is_null() {
            return null_mut();
        }
        ptr::copy_nonoverlapping(p, new, old.min(size));
        free(p);
        new
    }
}

unsafe fn cstr_len(s: *const u8) -> usize {
    let mut n = 0;
    unsafe {
        while ptr::read_volatile(s.add(n)) != 0 {
            n += 1;
        }
    }
    return n;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    unsafe {
        return cstr_len(s);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcmp(string1 : *const u8, string2 : *const u8) -> i32 {
    let mut i = 0;
    loop {
        let (x, y) = unsafe { (*string1.add(i), *string2.add(i)) };
        if x != y || x == 0 {
            return (x as i32) - (y as i32)
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncmp(string1: *const u8, string2: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let (x, y) = unsafe { (*string1.add(i), *string2.add(i))};
        if x != y || x == 0 {
            return (x as i32) - (y as i32);
        }
    }

    return 0;
}

fn lower(c: u8) -> u8 {
    if c.is_ascii_uppercase() { 
        return c + 32;
    }
    return c;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcasecmp(string1: *const u8, string2: *const u8) -> i32 {
    //literally the same as the other comparison one, but caseless, which is pretty cool
    let mut i = 0;
    loop {
        let (x, y) = unsafe { (lower(*string1.add(i)), lower(*string2.add(i))) };
        if x != y || x == 0 {
            return (x as i32) - (y as i32)
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncasecmp(string1: *const u8, string2: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let (x, y) = unsafe { (lower(*string1.add(i)), lower(*string2.add(i)))};
        if x != y || x == 0 {
            return (x as i32) - (y as i32);
        }
    }

    return 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strchr(string: *const u8, c: i32) -> *mut u8 {
    let c = c as u8;
    let mut p = string;
    unsafe {
        loop {
            if *p == c {
                return p as *mut u8;
            }
            if *p == 0 {
                return ptr::null_mut();
            }
            p = p.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strrchr(string: *const u8, c: i32) -> *mut u8 {
    let c = c as u8;
    let mut last: *const u8 = ptr::null();
    let mut p = string;
    unsafe {
        loop {
            if *p == c {
                last = p;
            }
            if *p == 0 {
                return last as *mut u8;
            }
            p = p.add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strdup(string: *const u8) -> *mut u8 {
    unsafe {
        let len = cstr_len(string);
        let p = malloc(len + 1);
        if !p.is_null() {
            ptr::copy_nonoverlapping(string, p, len + 1);
        }
        return p;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn toupper(c : i32) -> i32 {
    if (c >= 97 && c < 123){
        return c - 32;
    }
    return c;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tolower(c : i32) -> i32 {
    if (c >= 65 && c < 91){
        return c + 32;
    }
    return c;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn abs(x : i32) -> i32 {
    if x < 0 {
        return x * -1;
    }
    return x;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn atoi(mut string: *const u8) -> i32 {
    let mut neg = false;
    let mut n: i32 = 0;
    unsafe {
        while matches!(*string, b' ' | b'\t' | b'\n' | b'\r') {
            string = string.add(1);
        }

        if *string == b'-' {
            neg = true;
            string = string.add(1);
        } 
        else if * string == b'+' {
            neg = false;
            string = string.add(1);
        }

        while (*string).is_ascii_digit() {
            n = n.wrapping_mul(10).wrapping_add((*string - b'0') as i32);
            string = string.add(1);
        }
    }
    if neg { 
        return n.wrapping_neg();
    } 
    else { 
        return n 
    }
}

#[unsafe(no_mangle)]
pub static mut errno: i32 = 0;

#[repr(C)]
pub struct KFile {
    kind: i32,
}

static mut STDIN_F: KFile = KFile { kind: 0 };
static mut STDOUT_F: KFile = KFile {kind : 1};
static mut STDERR_F: KFile = KFile { kind: 2 };

#[unsafe(no_mangle)]
pub static mut stdin: *mut KFile = &raw mut STDIN_F;

#[unsafe(no_mangle)]
pub static mut stdout: *mut KFile = &raw mut STDOUT_F;

#[unsafe(no_mangle)]
pub static mut stderr: *mut KFile = &raw mut STDERR_F;

#[unsafe(no_mangle)]
pub extern "C" fn fopen(_path: *const u8, _mode: *const u8) -> *mut KFile {
    return null_mut();
}

#[unsafe(no_mangle)]
pub extern "C" fn fclose(_path: *const u8, _mode: *const u8) -> i32 {
    return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn mkdir(_path: *const u8, _mode: *const u8) -> *mut KFile {
    return null_mut();
}

#[unsafe(no_mangle)]
pub extern "C" fn putchar(c: i32) -> i32 {
    serial_write(c as u8);
    return c;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn puts(s: *const u8) -> i32 {
    unsafe { 
        serial_puts(s); 
    }
    serial_write(b'\n');
    return 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const u8) -> i32 {
    unsafe { 
        serial_puts(fmt); 
    }
    return 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn snprintf(buffer: *mut u8, n: usize, fmt: *const u8) -> i32 {
    unsafe {
        let len = cstr_len(fmt);
        if n > 0 {
            let c = len.min(n - 1);
            ptr::copy_nonoverlapping(fmt, buffer, c);
            *buffer.add(c) = 0;
        }
        return (len as i32);
    }
}