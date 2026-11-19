#![no_std]
#![no_main]

extern crate alloc;

use core::{panic::PanicInfo, ptr::read};
use core::sync::atomic::{AtomicU64, Ordering, AtomicBool};
use alloc::{boxed::Box, vec::Vec};
use linked_list_allocator::LockedHeap;
use bootloader::BootInfo;
use spin::Mutex;

mod libc;

#[derive(Clone, Copy)]
struct KeyEvent {
    pressed : bool,
    extended : bool,
    code : u8,
}

const queueLength : usize = 64;

struct KeyQueue {
    buffer : [Option<KeyEvent>; queueLength],
    head : usize,
    tail : usize,
}

impl KeyQueue {
    const fn new() -> Self {
        return KeyQueue {
            buffer : [None; queueLength], head : 0, tail : 0 
        }
    }

    fn push(&mut self, newEvent : KeyEvent) {
        let next = (self.tail + 1) % queueLength;

        if next != self.head {
            self.buffer[self.tail] = Some(newEvent);
            self.tail = next;
        }
    }

    fn pop(&mut self) -> Option<KeyEvent> {
        if self.head == self.tail {
            return None;
        }

        let keyEvent = self.buffer[self.head].take();

        self.head = (self.head + 1) % queueLength;

        return keyEvent;
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static TSC_PER_MS: AtomicU64 = AtomicU64::new(1);
static BOOT_TSC: AtomicU64 = AtomicU64::new(0);

const HEAP_SIZE: usize = 16 * 1024 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

static KEYS: Mutex<KeyQueue> = Mutex::new(KeyQueue::new());
static EXTENDED : AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_boot_info: &'static mut bootloader::BootInfo) -> ! {
    init_heap();
    libc::serial_init();
    init_timer();
    set_mode_13h();

    unsafe { core::ptr::write_bytes(0xA0000 as *mut u8, 0, 320 * 200); }

    static ARG0: [u8; 5] = *b"doom\0";
    let mut argv: [*mut u8; 2] = [ARG0.as_ptr() as *mut u8, core::ptr::null_mut()];

    unsafe {
        doomgeneric_Create(1, argv.as_mut_ptr());
        loop {
            doomgeneric_Tick();
        }
    }
}

unsafe extern "C" {
    fn doomgeneric_Create(argc: i32, argv: *mut *mut u8);
    fn doomgeneric_Tick();
    static mut DG_ScreenBuffer: *mut u8;
}

#[unsafe(no_mangle)]
pub extern "C" fn DG_SetPalette(palette: *const u8) {
    unsafe {
        outb(0x3C8, 0);                       // start at palette index 0
        for i in 0..768 {
            outb(0x3C9, *palette.add(i) >> 2);    // 8-bit to the DAC's 6 bits
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn DG_Init() {}

#[unsafe(no_mangle)]
pub extern "C" fn DG_DrawFrame() {
    unsafe {
        core::ptr::copy_nonoverlapping(DG_ScreenBuffer, 0xA0000 as *mut u8, 320 * 200);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn DG_SleepMs(ms: u32) {
    sleep_ms(ms.into());
}

#[unsafe(no_mangle)]
pub extern "C" fn DG_GetTicksMs() -> u32 {
    return ticks_ms() as u32; //returns u64 GOD IM SO GOOD AT THIS
}

#[unsafe(no_mangle)]
pub extern "C" fn DG_GetKey() -> u32 {
    return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn DG_SetWindowTitle(_title: *const u8) {
    //I wouldnt have the foggiest on how to do that so we're just hoping its not important :)
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

fn poll_keyboard() {
    unsafe {
        while keyboard_has_data() {
            let byte = inb(0x60);

            if byte == 0xE0 {
                EXTENDED.store(true, Ordering::Relaxed);
                continue;
            }

            let extended = EXTENDED.swap(false, Ordering::Relaxed);

            KEYS.lock().push(KeyEvent {
                pressed : byte & 0x80 == 0,
                extended : extended,
                code : byte & 0x7F 
            });
        }
    }
}

fn pop_keys() -> Option<KeyEvent> {
    return KEYS.lock().pop();
}

