use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open("./rom/hw.nes")?;
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf)?;
    println!("{:?}", buf);
    Ok(())
}
