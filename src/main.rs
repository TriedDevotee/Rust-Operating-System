#![no_std]
#![no_main]

extern crate alloc;

use core::{panic::PanicInfo, ptr::read};
use core::sync::atomic::{AtomicU64, Ordering};
use alloc::{boxed::Box, vec::Vec};
use linked_list_allocator::LockedHeap;
use bootloader::BootInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static TSC_PER_MS: AtomicU64 = AtomicU64::new(1);
static BOOT_TSC: AtomicU64 = AtomicU64::new(0);

const HEAP_SIZE: usize = 4 * 1024 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[unsafe(no_mangle)]
pub extern "C" fn _start(_boot_info: &'static mut bootloader::BootInfo) -> ! {
    init_heap();
    init_timer();
    set_mode_13h();

        set_palette_entry(0, 0, 0, 24);          // night sky
    set_palette_entry(2, 20, 90, 30);        // wire
    set_palette_entry(3, 255, 255, 255);     // snow

    // background
    for y in 0..200 { for x in 0..320 { put_pixel(x, y, 0); } }

    // snow: 90 pseudo-random specks
    for i in 0..90u64 {
        let h = hash(i);
        put_pixel((h % 320) as usize, ((h >> 16) % 200) as usize, 3);
    }

    draw_strand(8, 0, 0);
    draw_strand(68, 40, 3);
    draw_strand(128, 0, 5);

    let mut last_frame = 0;
    loop {
        let t = ticks_ms();
        if t - last_frame >= 25 {
            last_frame = t;
            update_lights(t);
        }
    }
}

static mut ROW : i32 = 0;
static mut COL : i32 = 0;

unsafe fn inb(port: u16) -> u8{
    let value : u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx")port, out("al")value);
    }
    value
}

unsafe fn outb(port : u16, val : u8){
    unsafe{
        core::arch::asm!("out dx, al", in("dx") port, in("al") val);
    }
}

fn init_heap() {
    unsafe {
        ALLOCATOR.lock().init(&raw mut HEAP as *mut u8, HEAP_SIZE);
    }
}

fn keyboard_has_data() -> bool {
    unsafe{
        let port = inb(0x64);
        return port & 1 != 0
    }
}

fn read_scancode() -> u8 {
    loop{
        if keyboard_has_data(){
            return unsafe { 
                inb(0x60) 
            }
        }
    }
}

const SCANCODE_MAP: [Option<char>; 128] = [
    None, None, Some('1'), Some('2'), Some('3'), Some('4'), Some('5'), Some('6'), Some('7'), Some('8'), Some('9'), Some('0'), Some('-'), Some('='), Some('\x08'), Some('\t'),
    Some('Q'), Some('W'), Some('E'), Some('R'), Some('T'), Some('Y'), Some('U'), Some('I'), Some('O'), Some('P'), Some('['), Some(']'), Some('\n'), None, Some('A'), Some('S'),
    Some('D'), Some('F'), Some('G'), Some('H'), Some('J'), Some('K'), Some('L'), Some(';'), Some('\''), Some('`'), None, Some('\\'), Some('Z'), Some('X'), Some('C'), Some('V'),
    Some('B'), Some('N'), Some('M'), Some(','), Some('.'), Some('/'), None, Some('*'), None, Some(' '), 

    //FUCKING REMAINING KEYS

    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, 
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, 
    None, None, None, None, None, None, None, None, None, None

];

fn scancode_to_char(code : u8) -> Option<char>{

    if code > 0x80 {
        return None;
    }

    SCANCODE_MAP[code as usize]
}

fn set_cursor_position(pos : u16){

    unsafe {
        outb(0x3D4, 0x0E);
        outb(0x3D5, (pos >> 8) as u8);
        outb(0x3D4, 0x0F);
        outb(0x3D5, (pos & 0xFF) as u8);
    }
}

fn get_cursor_position() -> u16 {
    unsafe{
        outb(0x3D4, 0x0E);
        let high = inb(0x3D5);
        outb(0x3D4, 0x0F);
        let low = inb(0x3D5);

        ((high as u16) << 8) | (low as u16)
    }
}

fn move_cursor_left() {
    let pos : u16 = get_cursor_position();
    if pos > 0 {
        set_cursor_position(pos - 1);
    }
}

fn move_cursor_right() {
    let pos : u16 = get_cursor_position();
    if pos < 1999 {
        set_cursor_position(pos + 1);
    }
}

fn vga_put(c : char){

    let pos  = get_cursor_position() as isize;
    
    let vga_buffer : *mut u8 = 0xb8000 as *mut u8;

    unsafe {
        *vga_buffer.offset(pos * 2) = c as u8;
        *vga_buffer.offset(pos * 2 + 1) = 0xb;

        move_cursor_right();
        COL = COL % 80; 
    }
}

fn backspace_key(){
    let pos = get_cursor_position() as isize;
    
    let vga_buffer : *mut u8 = 0xb8000 as *mut u8;

    unsafe {

        move_cursor_left();
        COL = COL % 80;

        *vga_buffer.offset(pos * 2) = 0x20;
        *vga_buffer.offset(pos * 2 + 1) = 0xb;
    }
}

fn set_mode_13h() {
    unsafe {
        let seq_regs: [u8; 5] = [0x03, 0x01, 0x0F, 0x00, 0x0E];
        let crtc_regs: [u8; 25] = [
            0x5F, 0x4F, 0x50, 0x82, 0x54,
            0x80, 0xBF, 0x1F, 0x00, 0x41,
            0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x9C, 0x0E, 0x8F, 0x28,
            0x40, 0x96, 0xB9, 0xA3, 0xFF,
        ];
        let gc_regs: [u8; 9] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x05, 0x0F, 0xFF];
        let ac_regs: [u8; 21] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0x41, 0x00, 0x0F, 0x00, 0x00,
        ];

        outb(0x3C2, 0x63);

        for (i, &v) in seq_regs.iter().enumerate() { 
            outb(0x3C4, i as u8); 
            outb(0x3C5, v); 
        }

        outb(0x3D4, 0x11);
        let v = inb(0x3D5);
        outb(0x3D5, v & 0x7F);

        for (i, &v) in crtc_regs.iter().enumerate() {
            outb(0x3D4, i as u8);
            outb(0x3D5, v);
        }

        for (i, &v) in gc_regs.iter().enumerate() {
            outb(0x3CE, i as u8);
            outb(0x3CF, v);
        }

        for (i, &v) in ac_regs.iter().enumerate() {
            let _ = inb(0x3DA);
            outb(0x3C0, i as u8);
            outb(0x3C0, v);
        }

        let _ = inb(0x3DA);
        outb(0x3C0, 0x20);
    }
}

fn set_palette_entry(i: u8, r: u8, g: u8, b: u8) {
    unsafe {
        outb(0x3C8, i);
        outb(0x3C9, r >> 2);
        outb(0x3C9, g >> 2);
        outb(0x3C9, b >> 2);
    }
}

fn put_pixel(x: usize, y: usize, color: u8) {
    if x >= 320 || y >= 200 { 
        return;
    }

    let vga = 0xA0000 as *mut u8;
    unsafe { 
        *vga.add(y * 320 + x) = color; 
    }
}

fn rdtsc() -> u64 {
    unsafe {
        return core::arch::x86_64::_rdtsc();
    }
}

fn init_timer() {
    unsafe {
        let v = inb(0x61);
        outb(0x61, (v & !0x02) | 0x01);

        outb(0x43, 0xB0);

        let count: u16 = 59659; //Thats the max value and then it counts down
        outb(0x42, (count & 0xFF) as u8);
        outb(0x42, (count >> 8) as u8);

        let start = rdtsc();
        while inb(0x61) & 0x20 == 0 {
            //This does nothing just counting, its pretty cool ngl
        }
        let end = rdtsc();

        TSC_PER_MS.store((end - start) / 50, Ordering::Relaxed)
    }
}

fn ticks_ms() -> u64 {
    let elapsed = rdtsc() - BOOT_TSC.load(Ordering::Relaxed);
    return elapsed / TSC_PER_MS.load(Ordering::Relaxed);
}

fn sleep_ms(ms: u64) {
    let target = ticks_ms() + ms;
    while ticks_ms() < target {
        //Again this just stalls the thread until ms has passed
    }
}

const COLOURS: [(u8, u8, u8); 8] = [
    (255, 0, 0), (0, 255, 0), (0, 120, 255), (255, 200, 0),
    (255, 0, 160), (255, 255, 255), (0, 255, 220), (255, 110, 0),
];

fn put_pixel_i(x: i32, y: i32, c: u8) {
    if x < 0 || y < 0 { return; }
    put_pixel(x as usize, y as usize, c);
}

fn fill_circle(cx: i32, cy: i32, r: i32, c: u8) {
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                put_pixel_i(cx + dx, cy + dy, c);
            }
        }
    }
}

// A drooping curve: repeats every 80 px, sags 32 px in the middle
fn wire_y(x: i32, base: i32) -> i32 {
    let u = (x % 80) - 40;
    base + (1600 - u * u) / 50
}

fn draw_strand(base: i32, xoff: i32, ch_off: i32) {
    for x in 0..320 {
        let y = wire_y(x + xoff, base);
        put_pixel_i(x, y, 2);
        put_pixel_i(x, y + 1, 2);
    }
    for n in 0..20 {
        let x = 8 + 16 * n;
        let y = wire_y(x + xoff, base);
        for s in 1..4 { put_pixel_i(x, y + s, 2); }   // little stem
        let ch = (n + ch_off) % 8;                    // which palette channel
        fill_circle(x, y + 7, 4, 16 + ch as u8);
    }
}

// Triangle wave: 0 -> 255 -> 0 over `period` milliseconds
fn tri(t: u64, period: u64) -> u64 {
    let p = t % period;
    let half = period / 2;
    let v = if p < half { p } else { period - p };
    v * 255 / half
}

// Cheap scrambler so "random" values are the same every run
fn hash(n: u64) -> u64 {
    let mut x = n.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 32;
    x = x.wrapping_mul(0xD6E8FEB86659FD93);
    x ^ (x >> 32)
}

fn scale(c: u8, b: u64) -> u8 {
    (c as u64 * b / 255) as u8
}

fn update_lights(t: u64) {
    let mode = (t / 8000) % 3;                 // change pattern every 8 seconds
    for k in 0..8u64 {
        let level = match mode {
            0 => tri(t + k * 100, 800),                          // wave rolls along
            1 => if hash(t / 250 * 8 + k) & 3 == 0 { 255 } else { 0 },  // twinkle
            _ => tri(t, 2000),                                   // all breathe together
        };
        let b = 50 + level * 205 / 255;        // never fully off: 50..255
        let (r, g, bl) = COLOURS[k as usize];
        set_palette_entry(16 + k as u8, scale(r, b), scale(g, b), scale(bl, b));
    }
}