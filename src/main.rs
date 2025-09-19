#![no_std]
#![no_main]

use core::{panic::PanicInfo, ptr::read};

use bootloader::BootInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_boot_info: &'static mut bootloader::BootInfo) -> ! {

    /* set_mode_13h();

    for index_y in 0..100 {
        for index_x in 0..100 {
            put_pixel(index_x, index_y, 15);
        }
    } */


    loop{
        if keyboard_has_data() {
            let code : u8 = read_scancode();
            let mut character = '?';

            if code < 0x80 {
                if let Some(c) = scancode_to_char(code) {
                    character = c;
                }
            }

            if character == '\x08' {
                backspace_key();
            } else if character != '?' {
                vga_put(character);
            }
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

fn keyboard_has_data() -> bool {
    unsafe{
        let port = inb(0x64);
        return port & 1 != 0
    }
}

fn read_scancode() -> u8 {
    loop{
        if keyboard_has_data(){
            return unsafe { inb(0x60) }
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
        let crtc11 = inb(0x3D5);
        outb(0x3D4, 0x11);
        outb(0x3D5, crtc11 & 0x7F);

        outb(0x3C2, 0x63);

        let seq_regs: [u8; 5] = [0x03, 0x01, 0x0F, 0x00, 0x0E];
        for (i, &val) in seq_regs.iter().enumerate() {
            outb(0x3C4, i as u8);
            outb(0x3C5, val);
        }

        let crtc_regs: [u8; 25] = [
            0x5F, 0x4F, 0x50, 0x82, 0x55,
            0x81, 0x8F, 0x1F, 0x00, 0x4F,
            0x0D, 0x0E, 0x00, 0x00, 0x00,
            0x9C, 0x0E, 0x8F, 0x28, 0x40,
            0x96, 0xB9, 0xA3, 0xFF, 0x00,
        ];
        for (i, &val) in crtc_regs.iter().enumerate() {
            outb(0x3D4, i as u8);
            outb(0x3D5, val);
        }

        let gc_regs: [u8; 9] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x05, 0x0F, 0xFF];
        for (i, &val) in gc_regs.iter().enumerate() {
            outb(0x3CE, i as u8);
            outb(0x3CF, val);
        }

        let ac_regs: [u8; 21] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0x41, 0x00, 0x0F, 0x00, 0x00,
        ];
        for (i, &val) in ac_regs.iter().enumerate() {
            let _ = inb(0x3DA); 
            outb(0x3C0, i as u8);
            outb(0x3C0, val);
        }

        let vga_buffer = 0xA0000 as *mut u8;
        for i in 0..(320 * 200) {
            *vga_buffer.offset(i) = 15; 
        }
    }
}

fn put_pixel(x: i32, y: i32, color: u8) {
    if x < 0 || x >= 320 || y < 0 || y >= 200 { return; }
    let offset = y * 320 + x;
    let vga_buffer = 0xA0000 as *mut u8;
    unsafe {
        *vga_buffer.offset(offset as isize) = color;
    }
}
