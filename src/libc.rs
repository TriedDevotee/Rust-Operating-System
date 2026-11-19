use alloc::alloc::{alloc as rust_alloc, dealloc as rust_dealloc, Layout};
use core::ptr::{self, null_mut};
use core::ffi::VaList;
use crate::{inb, outb};

static WAD_DATA : &[u8] = include_bytes!("../assets/DOOM1.WAD");

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
    if c >= 65 && c < 91{
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
    data: *const u8,
    len: usize,
    pos: usize,
}

static mut STDIN_F: KFile = KFile { 
    kind: 0,
    data: ptr::null(),
    len: 0,
    pos: 0
};

static mut STDOUT_F: KFile = KFile { 
    kind: 1,
    data: ptr::null(),
    len: 0,
    pos: 0
};

static mut STDERR_F: KFile = KFile { 
    kind: 2,
    data: ptr::null(),
    len: 0,
    pos: 0
};

static mut WAD_F: KFile = KFile {
    kind: 3,
    data: ptr::null(),
    len: 0,
    pos: 0
}

#[unsafe(no_mangle)]
pub static mut stdin: *mut KFile = &raw mut STDIN_F;

#[unsafe(no_mangle)]
pub static mut stdout: *mut KFile = &raw mut STDOUT_F;

#[unsafe(no_mangle)]
pub static mut stderr: *mut KFile = &raw mut STDERR_F;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fopen(
    path: *const u8,
    _mode: *const u8
) -> *mut KFile {
    if path.is_null() {
        return null_mut();
    }

    unsafe {
        if strcasecmp(path, b"DOOM1.WAD\0".as_ptr()) != 0 {
            return null_mut();
        }

        WAD_F.data = WAD_DATA.as_ptr();
        WAD_F.len = WAD_DATA.len();
        WAD_F.pos = 0;

        return &raw mut WAD_F;
    }
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
pub extern "C" fn exit(code: i32) {
    unsafe{
        let message = alloc::format!("ERROR: {}", code);
        serial_puts(message.as_ptr());
        loop {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn system() -> i32 {
    return -1;
}

#[unsafe(no_mangle)]
pub extern "C" fn remove() -> i32 {
    unsafe {
        errno = 2;
        return -1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rename() -> i32 {
    unsafe {
        errno = 2;
        return -1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fflush() -> i32 {
    return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn fread() -> i32 {
    return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn fseek() -> i32 {
    return -1;
}

#[unsafe(no_mangle)]
pub extern "C" fn ftell() -> i32 {
    return -1;
}


#[unsafe(no_mangle)]
pub unsafe extern "C" fn fwrite(pointer: *const u8, size: usize, count: usize, stream: *mut KFile) -> usize {
    if pointer.is_null() || stream.is_null() {
        return 0;
    }

    if size == 0 || count == 0 {
        return 0;
    }

    let total = match size.checked_mul(count) {
        Some(n) => n,
        None => return 0,
    };

    unsafe {
        match (*stream).kind {
            1 | 2 => {
                for i in 0..total {
                    serial_write(*pointer.add(i));
                }
            }
            _ => return 0,
        }
    }

    count
}

#[unsafe(no_mangle)]
pub extern "C" fn sscanf(_s: *const u8, _fmt: *const u8) -> i32 { 
    return 0; 
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    unsafe {
        while i < n && *src.add(i) != 0 {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
        while i < n {
            *dest.add(i) = 0;
            i += 1;
        }
    }
    return dest;
}

//-----------NOTE TO READER THIS IS CLAUDES CODE I HAVE
//-----------LITERALLY NO IDEA HOW THE FUCK THIS WORKS BUT I JUST NEED IT TO
//-----------THIS WILL BE REWRITTEN AT A LATER DATE, I JUST WANT TO GET IT WORKING

unsafe fn arg_i32(ap: &mut VaList<'_>) -> i32 { unsafe { ap.next_arg::<i32>() } }
unsafe fn arg_i64(ap: &mut VaList<'_>) -> i64 { unsafe { ap.next_arg::<i64>() } }
unsafe fn arg_ptr(ap: &mut VaList<'_>) -> *const u8 { unsafe { ap.next_arg::<*const u8>() } }

// ---- output sink: counts every byte and forwards it ----
struct Out<'a> {
    f: &'a mut dyn FnMut(u8),
    count: i32,
}

impl Out<'_> {
    fn put(&mut self, b: u8) {
        (self.f)(b);
        self.count += 1;
    }
    fn pad(&mut self, ch: u8, n: usize) {
        for _ in 0..n {
            self.put(ch);
        }
    }
}

// ---- integer output: sign, prefix, zeros, digits, padding ----
fn emit_int(
    out: &mut Out<'_>,
    mut mag: u64,
    neg: bool,
    base: u64,
    upper: bool,
    prefix: &[u8],
    left: bool,
    zero: bool,
    width: usize,
    prec: Option<usize>,
) {
    let mut digits = [0u8; 24];
    let mut n = 0;

    if mag == 0 && prec != Some(0) {
        digits[0] = b'0';
        n = 1;
    }
    while mag > 0 {
        let d = (mag % base) as u8;
        digits[n] = if d < 10 {
            b'0' + d
        } else if upper {
            b'A' + d - 10
        } else {
            b'a' + d - 10
        };
        mag /= base;
        n += 1;
    }

    let sign: &[u8] = if neg { b"-" } else { b"" };
    let prec_zeros = prec.map_or(0, |p| p.saturating_sub(n));
    let body = sign.len() + prefix.len() + prec_zeros + n;
    let pad = width.saturating_sub(body);
    let zero_pad = zero && !left && prec.is_none();   // '-' and a precision both cancel '0'

    if !left && !zero_pad {
        out.pad(b' ', pad);
    }
    for &b in sign {
        out.put(b);
    }
    for &b in prefix {
        out.put(b);
    }
    if zero_pad {
        out.pad(b'0', pad);
    }
    out.pad(b'0', prec_zeros);
    for i in (0..n).rev() {
        out.put(digits[i]);
    }
    if left {
        out.pad(b' ', pad);
    }
}

// ---- the engine ----
unsafe fn format(f: &mut dyn FnMut(u8), mut fmt: *const u8, ap: &mut VaList<'_>) -> i32 {
    let mut out = Out { f, count: 0 };

    unsafe {
        loop {
            let c = *fmt;
            if c == 0 {
                break;
            }
            fmt = fmt.add(1);
            if c != b'%' {
                out.put(c);
                continue;
            }

            // flags
            let mut left = false;
            let mut zero = false;
            loop {
                match *fmt {
                    b'-' => left = true,
                    b'0' => zero = true,
                    b'+' | b' ' | b'#' => {}   // accepted, ignored
                    _ => break,
                }
                fmt = fmt.add(1);
            }

            // width
            let mut width = 0usize;
            if *fmt == b'*' {
                fmt = fmt.add(1);
                let w = arg_i32(ap);
                if w < 0 {
                    left = true;
                }
                width = w.unsigned_abs() as usize;
            } else {
                while (*fmt).is_ascii_digit() {
                    width = width * 10 + (*fmt - b'0') as usize;
                    fmt = fmt.add(1);
                }
            }

            // precision
            let mut prec: Option<usize> = None;
            if *fmt == b'.' {
                fmt = fmt.add(1);
                if *fmt == b'*' {
                    fmt = fmt.add(1);
                    let p = arg_i32(ap);
                    if p >= 0 {
                        prec = Some(p as usize);
                    }
                } else {
                    let mut p = 0usize;
                    while (*fmt).is_ascii_digit() {
                        p = p * 10 + (*fmt - b'0') as usize;
                        fmt = fmt.add(1);
                    }
                    prec = Some(p);
                }
            }

            // length modifiers: l, ll, z, j, t are all 64-bit here; h and hh truncate
            let mut long = false;
            let mut hcount = 0u8;
            loop {
                match *fmt {
                    b'l' | b'z' | b'j' | b't' => long = true,
                    b'h' => hcount += 1,
                    _ => break,
                }
                fmt = fmt.add(1);
            }

            // conversion
            let conv = *fmt;
            if conv == 0 {
                break;   // a lone '%' at the very end
            }
            fmt = fmt.add(1);

            match conv {
                b'd' | b'i' => {
                    let v: i64 = if long {
                        arg_i64(ap)
                    } else {
                        let v = arg_i32(ap);
                        match hcount {
                            0 => v as i64,
                            1 => v as i16 as i64,
                            _ => v as i8 as i64,
                        }
                    };
                    emit_int(&mut out, v.unsigned_abs(), v < 0, 10, false, b"", left, zero, width, prec);
                }
                b'u' | b'x' | b'X' => {
                    let v: u64 = if long {
                        arg_i64(ap) as u64
                    } else {
                        let v = arg_i32(ap) as u32;
                        match hcount {
                            0 => v as u64,
                            1 => v as u16 as u64,
                            _ => v as u8 as u64,
                        }
                    };
                    let (base, upper) = match conv {
                        b'u' => (10, false),
                        b'x' => (16, false),
                        _ => (16, true),
                    };
                    emit_int(&mut out, v, false, base, upper, b"", left, zero, width, prec);
                }
                b'p' => {
                    let v = arg_ptr(ap) as usize as u64;
                    emit_int(&mut out, v, false, 16, false, b"0x", left, zero, width, prec);
                }
                b'c' => {
                    let ch = arg_i32(ap) as u8;
                    let pad = width.saturating_sub(1);
                    if !left {
                        out.pad(b' ', pad);
                    }
                    out.put(ch);
                    if left {
                        out.pad(b' ', pad);
                    }
                }
                b's' => {
                    let p = arg_ptr(ap);
                    let s: *const u8 = if p.is_null() { b"(null)\0".as_ptr() } else { p };
                    let limit = prec.unwrap_or(usize::MAX);
                    let mut len = 0usize;
                    while len < limit && *s.add(len) != 0 {
                        len += 1;
                    }
                    let pad = width.saturating_sub(len);
                    if !left {
                        out.pad(b' ', pad);
                    }
                    for i in 0..len {
                        out.put(*s.add(i));
                    }
                    if left {
                        out.pad(b' ', pad);
                    }
                }
                b'%' => out.put(b'%'),
                _ => {
                    out.put(b'%');   // unknown conversion: show it as typed
                    out.put(conv);
                }
            }
        }
    }
    out.count
}

// ---- the public functions: thin wrappers choosing a sink ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vsnprintf(buf: *mut u8, n: usize, fmt: *const u8, mut ap: VaList<'_>) -> i32 {
    let mut written = 0usize;
    let total = unsafe {
        format(
            &mut |b| {
                if written + 1 < n {
                    *buf.add(written) = b;
                }
                written += 1;
            },
            fmt,
            &mut ap,
        )
    };
    if n > 0 {
        unsafe { *buf.add(written.min(n - 1)) = 0; }
    }
    total
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn snprintf(buf: *mut u8, n: usize, fmt: *const u8, mut args: ...) -> i32 {
    // on older nightlies this may need `args.as_va_list()` instead of `args`
    unsafe { vsnprintf(buf, n, fmt, args) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const u8, mut args: ...) -> i32 {
    unsafe { format(&mut |b| serial_write(b), fmt, &mut args) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vfprintf(stream: *mut KFile, fmt: *const u8, mut ap: VaList<'_>) -> i32 {
    let to_serial = !stream.is_null() && matches!(unsafe { (*stream).kind }, 1 | 2);
    unsafe {
        format(
            &mut |b| {
                if to_serial {
                    serial_write(b);
                }
            },
            fmt,
            &mut ap,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fprintf(stream: *mut KFile, fmt: *const u8, mut args: ...) -> i32 {
    unsafe { vfprintf(stream, fmt, args) }
}