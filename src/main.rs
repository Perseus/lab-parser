use std::fs::File;
use std::thread;
use std::path::Path;
use anim::bone::lwAnimDataBone;
use byteorder::{LittleEndian, ReadBytesExt};
use std::time::Duration;
use std::sync::mpsc;
use ui::opengl;
use glium::*;
use cgmath::*;

mod anim;
mod ui;



pub const MIN_VERSION: u16 = 4010;

fn main() {
    let path = Path::new("./0912.lab");
    let display = path.display();

    let mut file = match File::open(&path) {
      Err(why) => panic!("Couldn\'t open {}: {}", display, why),
      Ok(file) => file,
    };
  
    let version =file.read_u16::<LittleEndian>().unwrap();
    
    if version < MIN_VERSION {
      println!("The animation file's version is too low");
      return
    }

    // lwAnimDataBone::load_from_file(&mut file);
    let mut anim_data = lwAnimDataBone::new();
    anim_data.load_from_file(&mut file);
    anim_data.write_joints_to_file();
}
