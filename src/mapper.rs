
trait Mapper {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, v: u8);
}

fn new_mapper(n : u8, prg : Vec::<u8>) -> dyn Mapper {
    Mapper0::new(prg)
    // match n {
    //     0 => Mapper0::new(prg),
    //     3 => Mapper3::new(prg),
    //     _ = > panic!("not impl %d", n)
    // }
}

struct Mapper0 {
    prg : Vec::<u8>
}

impl Mapper0 {
    fn new(prg : Vec::<u8>) -> Self {
        Self {
           prg: prg
        }
    }
}

impl Mapper for Mapper1 {
    fn read(&self, addr: u16) -> u8{
        self.prg[addr as usize]
    }
    fn write(&mut self, addr: u16, v: u8) {
    }
}

struct Mapper3 {
    prg : Vec::<u8>,
    bank : usize
}

impl Mapper3 {
    fn new(prg : Vec::<u8>) -> Self {
        Self {
           prg: prg,
           bank: 0,
        }
    }
}

impl Mapper for Mapper3 {
    fn read(&self, addr: u16) -> u8{
        self.prg[addr as usize | self.bank]
    }
    fn write(&mut self, addr: u16, v: u8) {
        //https://www.nesdev.org/wiki/INES_Mapper_003#Bank_select_($8000-$FFFF)
        self.bank = (0x03 & (v as usize)) << 12;
    }
}