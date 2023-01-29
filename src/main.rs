use famiko::FamikoOption;
use hex::decode;

use std::fs::File;
use std::io::Read;
use clap::{arg, Command, Arg, ArgAction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("famiko")
        .arg(arg!(--start_addr [addr] "開始アドレス"))
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .action(ArgAction::SetTrue)
                .help("デバッグログON")
        )
        .arg(
            Arg::new("sound-debug")
                .long("sound-debug")
                .action(ArgAction::SetTrue)
                .help("サウンドのデバッグ出力ON")
        )
        .arg(Arg::new("no-sound").long("no-sound").action(ArgAction::SetTrue))
        .arg(
            Arg::new("show-chr-table")
                .long("show-chr-table")
                .action(ArgAction::SetTrue)
                .help("キャラクタテーブル表示")
        )
        .arg(
            Arg::new("show-name-table")
                .long("show-name-table")
                .action(ArgAction::SetTrue)
                .help("ネームテーブル表示")
        )
        .arg(
            Arg::new("show-sprite")
                .long("show-sprite")
                .action(ArgAction::SetTrue)
                .help("スプライトテーブル表示")
        )
        .arg(
            Arg::new("fps")
                .long("fps")
                .action(ArgAction::SetTrue)
                .help("fps出力")
        )
        .arg(arg!([rom] "rom").help("ROMファイル"))
        .get_matches();

    let option = FamikoOption {
        start_addr: if let Some(data) = matches.get_one::<String>("start_addr") {
            let v = decode(data).unwrap();
            let addr = ((v[0] as u16) << 8) | (v[1] as u16);
            Some(addr)
        } else {
            None
        },
        debug: matches.get_one::<bool>("debug").map_or(false, |v| *v),
        sound_debug: matches.get_one::<bool>("sound-debug").map_or(false, |v| *v),
        no_sound: matches.get_one::<bool>("no-sound").map_or(false, |v| *v),
        show_chr_table: matches.get_one::<bool>("show-chr-table").map_or(false, |v| *v),
        show_name_table: matches.get_one::<bool>("show-name-table").map_or(false, |v| *v),
        show_sprite: matches.get_one::<bool>("show-sprite").map_or(false, |v| *v),
        is_show_fps: matches.get_one::<bool>("fps").map_or(false, |v| *v),
        rom_bytes : {
            let file = matches.get_one::<String>("rom").unwrap();
            let mut file = File::open(file)?;
            let mut rom = include_bytes!("../rom/smb.nes").to_vec();
            let _ = file.read_to_end(&mut rom)?;
            // println!("{:?}", rom);
            rom
        }
    };

    
    famiko::main(&option)
}