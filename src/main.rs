use std::fs::File;
use std::io::Read;
use pretty_hex::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open("./rom/hw.nes")?;
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf)?;
    // println!("{:?}", buf);

    let h = parse_header(&buf).unwrap();

    let mut p : usize = 16;
    let prg_rom = &buf[p .. p + h.prg_size];
    p += h.prg_size;
    let chr_rom = &buf[p .. p+h.chr_size];

    println!("{:?}", h);
    println!("{:?}", prg_rom.hex_dump());
    println!("{:?}", chr_rom.hex_dump());

    Ok(())
}

#[derive(Debug)]
struct NesHeader {
    prg : u8,
    prg_size : usize,
    chr : u8,
    chr_size : usize,
    flag6 : u8,
    // flag6
    // 76543210
    // ||||||||
    // |||||||+- Mirroring: 0: horizontal (vertical arrangement) (CIRAM A10 = PPU A11)
    // |||||||              1: vertical (horizontal arrangement) (CIRAM A10 = PPU A10)
    // ||||||+-- 1: Cartridge contains battery-backed PRG RAM ($6000-7FFF) or other persistent memory
    // |||||+--- 1: 512-byte trainer at $7000-$71FF (stored before PRG data)
    // ||||+---- 1: Ignore mirroring control or above mirroring bit; instead provide four-screen VRAM
    // ++++----- Lower nybble of mapper number
    trainer_exist : bool,
}

fn parse_header(buf : &[u8]) -> Result<Box<NesHeader>, Box<dyn std::error::Error>> {
    
    if buf.len() < 4 {
        panic!("header size error");
    }

    if buf[0] != 'N' as u8 || buf[1] != 'E' as u8 || buf[2] != 'S' as u8 || buf[3] != 0x1A {
        panic!("constant bytes error");
    }

    let prg = buf[4];
    let chr = buf[5];
    let flag6 = buf[6];



    Ok(Box::new(NesHeader{
        prg : prg,
        prg_size: prg as usize * 16 * 1024,
        chr : chr,
        chr_size: chr as usize * 8 * 1024,
        flag6 : flag6,
        trainer_exist : flag6 & 0x40 != 0
    }))
}