use anim::bone::AnimDataBone;
use byteorder::{LittleEndian, ReadBytesExt};
use std::{env, fs::File};
use std::fs::*;
use std::io::prelude::*;
use std::path::Path;
use std::ffi::OsStr;

mod anim;

pub const MIN_VERSION: u16 = 4010;

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        panic!("Provide at least two arguments to the application.");
    }

    let operation = &args[1];
    match operation.as_str() {
        "lab2dae" => {
             match get_extension_from_filename(&args[2]) {
                Some(extension) => {
                    if extension != "lab" {
                        panic!("Can't read non .lab file");
                    }
                },

                None => panic!("Unrecognized file format")
            };

            let lab_file_path = Path::new(&args[2]);
            let file_stem = OsStr::to_str(lab_file_path.file_stem().unwrap()).unwrap();
            let display = lab_file_path.display();
            let mut file = match File::open(&lab_file_path) {
                Err(why) => panic!("Couldn't open {}: {}", display, why),
                Ok(file) => file,
            };

            let version = file.read_u16::<LittleEndian>().unwrap();
            if version < MIN_VERSION {
                panic!("The animation file's version is incompatible with this program");
            }

            let mut anim_data = AnimDataBone::new();
            println!("Loading animation data...");
            let xml_content = anim_data.load_from_file(&mut file);
            let result_file_name = &format!("./{}.dae",file_stem);
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(result_file_name).unwrap();
            println!("Writing data to a collada file...");

            file.write_all("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n".as_bytes()).unwrap();
            file.write_all(xml_content.as_bytes()).unwrap();
            println!("Done!");
        },
        "dae2lab" => {
            println!("This operation is currently not supported");
        },
        _ => {
            println!("Unknown operation specified");
        }
    };
}
