use anim::bone::AnimDataBone;
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::fs::*;
use std::io::prelude::*;
use std::path::Path;
use collada::{document::ColladaDocument};
mod anim;

pub const MIN_VERSION: u16 = 4010;

fn main() {
    let path = Path::new("./BirdBitchTest.dae");
    let document = ColladaDocument::from_path(path);

    match document {
        Ok(doc) => {
            println!("{:?}", doc.get_animations().unwrap());
        },

        Err(err) => {
            panic!(format!("Couldn't parse collada file. Error - {}", err));
        }
    }

    /*
    let display = path.display();

    let mut file = match File::open(&path) {
        Err(why) => panic!("Couldn\'t open {}: {}", display, why),
        Ok(file) => file,
    };
    let version = file.read_u16::<LittleEndian>().unwrap();

    if version < MIN_VERSION {
        println!("The animation file's version is too low");
        return;
    }

    // AnimDataBone::load_from_file(&mut file);
    let mut anim_data = AnimDataBone::new();
    println!("Loading animation data...");
    let xml_content = anim_data.load_from_file(&mut file);
    let mut file = OpenOptions::new()
                              .write(true)
                              .create(true)
                              .open("./model.dae").unwrap();
    println!("Writing data to a collada file...");
    file.write_all("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n".as_bytes()).unwrap();
    file.write_all(xml_content.as_bytes()).unwrap();
    println!("Done!");
    */


}
