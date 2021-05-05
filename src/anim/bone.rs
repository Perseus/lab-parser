use std::{borrow::{Borrow, BorrowMut}, fs::File, io::{ Read }, mem::size_of, thread::current, u32};
use std::io::{ Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use cgmath::{Matrix, Matrix4, Quaternion, SquareMatrix, Transform, Vector3};
use serde::{ Serialize, Deserialize };
use crate::ui::camera::{*, self};
use xmlwriter::*;
use std::collections::HashMap;
use std::cell::RefCell;

use super::d3d::{lwMatrix43, lwMatrix44};

#[derive(Debug)]
enum lwBoneInfoKeyType {
  BoneKeyTypeMat43 = 1,
  BoneKeyTypeMat44,
  BoneKeyTypeQuat,
  BoneKeyTypeInvalid
}
#[derive(Debug)]
struct lwBoneInfoHeader {
  bone_num: u32,
  frame_num: u32,
  dummy_num: u32,
  key_type: lwBoneInfoKeyType,
}
#[derive(Debug)]
struct lwBoneBaseInfo {
  name: [u8; 64],
  id: u32,
  parent_id: u32,
}

#[derive(PartialEq, Debug)]
struct lwBoneDummyInfo {
  id: u32,
  parent_bone_id: u32,
  mat: lwMatrix44,
}

#[derive(Debug, Clone)]
struct lwBoneKeyInfo {
  mat43_seq: Option<Vec<lwMatrix43>>,
  mat44_seq: Option<Vec<lwMatrix44>>,
  pos_seq: Option<Vec<Vector3<f32>>>,
  quat_seq: Option<Vec<Quaternion<f32>>>
}

#[derive(Debug)]
pub struct lwAnimDataBone<'a> {
  header: lwBoneInfoHeader,
  base_seq: Vec<lwBoneBaseInfo>,
  dummy_seq: Vec<lwBoneDummyInfo>,
  key_seq: Vec<lwBoneKeyInfo>,
  invmat_seq: Vec<lwMatrix44>,
  
  joints: Option<joint<'a>>,
  bones: HashMap<u32, RefCell<joint<'a>>>,
}

#[derive(Debug)]
struct joint<'a> {
  parent: Option<&'a RefCell<joint<'a>>>,
  children: Vec<&'a RefCell<joint<'a>>>,
  bone_id: u32,
  bone_name: String,
  parent_id: u32,
  transformation_matrix: Option<Matrix4<f32>>,
}

impl lwBoneInfoHeader {
  pub fn new() -> lwBoneInfoHeader {
    lwBoneInfoHeader {
      bone_num: 0,
      frame_num: 0,
      dummy_num: 0,
      key_type: lwBoneInfoKeyType::BoneKeyTypeInvalid,
    }
  }
}

impl lwBoneBaseInfo {
  pub fn new() -> lwBoneBaseInfo {
    lwBoneBaseInfo {
      name: [0; 64],
      id: 0,
      parent_id: 0,      
    }
  }

  pub fn get_name(&self) -> String {
    let mut name_vec: Vec<u8> = Vec::new();
    for i in self.name.iter() {
      if *i == ('\0' as u8) {
        break;
      }

      name_vec.push(*i);
    }
    return String::from_utf8(name_vec).unwrap();
  }
}

impl lwBoneDummyInfo {
  pub fn new() -> lwBoneDummyInfo {
    lwBoneDummyInfo {
      id: 0,
      parent_bone_id: 0,
      mat: lwMatrix44::new([[0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]]),
    }
  }
}

impl lwBoneKeyInfo {
  pub fn new() -> lwBoneKeyInfo {
    lwBoneKeyInfo {
      mat43_seq: None,
      mat44_seq: None,
      pos_seq: None,
      quat_seq: None   
    }
  }
}


impl<'a> lwAnimDataBone<'a> {
  pub fn new() -> lwAnimDataBone<'a> {
    lwAnimDataBone {
      header: lwBoneInfoHeader::new(),
      base_seq: Vec::new(),
      dummy_seq: Vec::new(),
      key_seq: Vec::new(),
      invmat_seq: Vec::new(),
      joints: None,
      bones: HashMap::new(),
    }
  }

  pub fn load_from_file(&mut self, file: &mut File) {
    file.seek(SeekFrom::Start(4)).unwrap();

    self.load_header(file);
    self.load_base_seq(file);
    self.load_invmat_seq(file);
    self.load_dummy_seq(file);
    self.load_key_seq(file);
  }

  fn load_header(&mut self, file: &mut File) {
    self.header.bone_num = file.read_u32::<LittleEndian>().unwrap();
    self.header.frame_num = file.read_u32::<LittleEndian>().unwrap();
    self.header.dummy_num = file.read_u32::<LittleEndian>().unwrap();
    let key_type = file.read_u32::<LittleEndian>().unwrap();

    self.header.key_type = match key_type {
      1 => {
        lwBoneInfoKeyType::BoneKeyTypeMat43
      },
      2 => {
        lwBoneInfoKeyType::BoneKeyTypeMat44
      },
      3 => {
        lwBoneInfoKeyType::BoneKeyTypeQuat
      },
      _ => {
        lwBoneInfoKeyType::BoneKeyTypeInvalid
      }
    };
  }

  fn load_base_seq(&mut self, file: &mut File) {
    for _ in 0..self.header.bone_num {
      let mut bone_seq = lwBoneBaseInfo::new();
      file.read_exact(&mut bone_seq.name).unwrap();
      bone_seq.id = file.read_u32::<LittleEndian>().unwrap();
      bone_seq.parent_id = file.read_u32::<LittleEndian>().unwrap();

      self.base_seq.push(bone_seq);
    }
  }


  fn load_invmat_seq(&mut self, file: &mut File) {

    for _ in 0..self.header.bone_num {
      let mut bytes: [u8; 64] = [0; 64];
      file.read_exact(&mut bytes).unwrap();

      let decoded: [[f32; 4]; 4] = bincode::deserialize(&bytes).unwrap();
      let invmat = lwMatrix44::new(decoded);

      self.invmat_seq.push(invmat);
    }
  }

   fn load_dummy_seq(&mut self, file: &mut File) {
    for _ in 0..self.header.dummy_num {
      let mut bytes: [u8; 64] = [0; 64];
      let id = file.read_u32::<LittleEndian>().unwrap();
      let parent_bone_id = file.read_u32::<LittleEndian>().unwrap();
      file.read_exact(&mut bytes).unwrap();

      let decoded: [[f32; 4]; 4] = bincode::deserialize(&bytes).unwrap();
      let dummy_info = lwBoneDummyInfo{
        id,
        parent_bone_id,
        mat: lwMatrix44::new(decoded),
      };

      self.dummy_seq.push(dummy_info);
    }
  }

  fn load_key_seq(&mut self, file: &mut File) {
    let mut keys = vec![lwBoneKeyInfo::new(); self.header.bone_num as usize];

    match self.header.key_type {
      lwBoneInfoKeyType::BoneKeyTypeMat43 =>  {
        for i in 0..self.header.bone_num {
          let key = &mut keys[i as usize];
          let mut mat43_seq_vec = vec![lwMatrix43{
            matrix: Default::default()
          }; self.header.frame_num as usize];

          let mut mat43_seq_bytes: Vec<u8> = vec![0; self.header.frame_num as usize];
          file.read_exact(&mut mat43_seq_bytes).unwrap();

          mat43_seq_vec[0] = bincode::deserialize(&mat43_seq_bytes).unwrap();
          key.mat43_seq = Some(mat43_seq_vec);
        }
      },

      lwBoneInfoKeyType::BoneKeyTypeMat44 => {
        for i in 0..self.header.bone_num {
          let key = &mut keys[i as usize];
          let mut mat44_seq_vec = vec![lwMatrix44::default(); self.header.frame_num as usize];

          let mut mat44_seq_bytes: Vec<u8> = vec![0; self.header.frame_num as usize];
          file.read_exact(&mut mat44_seq_bytes).unwrap();

          let decoded: [[f32; 4]; 4] = bincode::deserialize(&mat44_seq_bytes).unwrap();
          mat44_seq_vec[0] = lwMatrix44::new(decoded);
          key.mat44_seq = Some(mat44_seq_vec);
        }
      },

      lwBoneInfoKeyType::BoneKeyTypeQuat => {
        for i in 0..self.header.bone_num {
          let key = &mut keys[i as usize];
          let mut pos_seq_vec = vec![Vector3::new(0.0, 0.0, 0.0); self.header.frame_num as usize];

          for j in 0..(self.header.frame_num as usize) {
            let mut pos_seq_bytes: Vec<u8> = vec![0; size_of::<Vector3<f32>>() ];
            file.read_exact(&mut pos_seq_bytes).unwrap();
            let deserialized: [f32; 3] = bincode::deserialize(&pos_seq_bytes).unwrap();
            pos_seq_vec[j] = Vector3::new(deserialized[0], deserialized[1], deserialized[2]);
          }

          key.pos_seq = Some(pos_seq_vec);

          let mut quat_seq_vec = vec![Quaternion::new(0.0, 0.0, 0.0, 0.0); self.header.frame_num as usize];

          for j in 0..(self.header.frame_num as usize) {
            let mut quat_seq_bytes: Vec<u8> = vec![0; size_of::<Quaternion<f32>>() ];
            file.read_exact(&mut quat_seq_bytes).unwrap();
            let deserialized: [f32; 4] =  bincode::deserialize(&quat_seq_bytes).unwrap();
            quat_seq_vec[j] = Quaternion::new(deserialized[3], deserialized[0], deserialized[1], deserialized[2]);
          }

          key.quat_seq = Some(quat_seq_vec);
        }
      },

      lwBoneInfoKeyType::BoneKeyTypeInvalid => {

      }
    };

    self.key_seq = keys;
  }


  pub fn get_transforms_for_frame(&self, frame: usize) -> Vec<Matrix4<f32>> {
    let mut finish_matrices: Vec<Matrix4<f32>>  = Vec::new();
    
    for i in 0..self.header.bone_num as usize {
      let key = &self.key_seq[i];
      let frame_quat = match &key.quat_seq {
        Some(e) => {
          e
        },
        None => {
          panic!("No frame_quat found");
        }
      };
      let frame_pos = match &key.pos_seq {
        Some(e) => {
          e
        },
        None => {
          panic!("No frame_pos found");
        }
      };
      let mut current_matrix: Matrix4<f32> = SquareMatrix::identity();


      match &self.header.key_type {
        lwBoneInfoKeyType::BoneKeyTypeQuat => {
          let quat: Quaternion<f32> = Quaternion::from(frame_quat[frame]);
          let offset: Vector3<f32> = Vector3::from(frame_pos[frame]);
          let mat1 = Matrix4::from(quat);
          let mat2 = Matrix4::from_translation(offset);
          current_matrix = Matrix4::from(quat) * Matrix4::from_translation(offset);
        },

        _ => {

        }
      }

      if self.base_seq[i].parent_id != u32::MAX && *&self.base_seq[i].parent_id < self.header.bone_num {
        current_matrix = current_matrix * finish_matrices[self.base_seq[i].parent_id as usize];
      }

      finish_matrices.push(current_matrix);
    }

    finish_matrices
  }

  fn create_joint_hierarchy(&'a mut self) {

    // create a map with some minimal information about all the bones, keyed by their ids
    for i in 0..self.get_num_bones() as usize {
      let bone_data = &self.base_seq[i];
      let current_node = joint{
        bone_id: bone_data.id,
        bone_name: bone_data.get_name(),
        parent: None,
        children: Vec::new(),
        parent_id: bone_data.parent_id,
        transformation_matrix: None
      };

      if !self.bones.contains_key(&bone_data.id) {
        self.bones.insert(bone_data.id, RefCell::new(current_node));
      }
    }

    for (_, v) in self.bones.iter() {
      let mut current_node = v.borrow_mut();
      let parent = self.bones.get(&current_node.parent_id);
      match parent {
        Some(p) => {
          current_node.parent = Some(p);
          p.borrow_mut().children.push(v);
        },
        None => {
        }
      }
    }

    for i in 0..self.get_num_bones() as usize {
      let key = &self.key_seq[i];
      let bone_data = &self.base_seq[i];
      let mut current_matrix:Matrix4<f32> = SquareMatrix::identity();

      let frame_quat = match &key.quat_seq {
        Some(e) => {
          e
        },
        None => {
          panic!("No frame_quat found");
        }
      };
      let frame_pos = match &key.pos_seq {
        Some(e) => {
          e
        },
        None => {
          panic!("No frame_pos found");
        }
      };

      match self.header.key_type {
        lwBoneInfoKeyType::BoneKeyTypeQuat => {
          let quat: Quaternion<f32> = Quaternion::from(frame_quat[0]);
          let offset: Vector3<f32> = Vector3::from(frame_pos[0]);
          current_matrix = Matrix4::from(quat) * Matrix4::from_translation(offset);
        },
        _ => {

        }
      }

      let joint = self.bones.get(&bone_data.id);
      match joint {
        Some(j) => {
          j.borrow_mut().transformation_matrix = Some(current_matrix);
        },
        None => {
          println!("No joint found for bone {}", i);
        }
      }
    }
  }

  pub fn apply_transforms(&self, transforms: Vec<Matrix4<f32>>) -> Vec<Vector3<f32>> {
    let mut positions: Vec<Vector3<f32>> = Vec::with_capacity(self.header.bone_num as usize);

    for i in 0..self.header.bone_num as usize {
      let inv_mat = Matrix4::from(self.invmat_seq[i].matrix);
      let original_mat = inv_mat.invert().unwrap();
      let start_pos = Vector3::new(original_mat[0][3], original_mat[1][3], original_mat[2][3]);
      let transformation_matrix = inv_mat * transforms[i];
      
      if i >= 8 {
        println!("{:?}", transformation_matrix);
      }
      positions.push(transformation_matrix.transform_vector(start_pos));
    }

    positions
  }


  pub fn get_num_bones(&self) -> u32 {
    self.header.bone_num
  }

  pub fn get_num_frames(&self) -> u32 {
    self.header.frame_num
  }

  pub fn add_vertices_to_vec(&'a mut self) {
    
    // self.create_joint_hierarchy();
  }

  pub fn write_joints_to_file(&'a mut self) {
    self.create_joint_hierarchy();

    let opt = Options {
      use_single_quote: false,
      ..Options::default()
    };

    let mut w = XmlWriter::new(opt);
    let joints = &self.joints;
    match joints {
      Some(j) => {
        lwAnimDataBone::write_node(*j, &mut w);
        // w.start_element("node");
        // lwAnimDataBone::write_ele_attrs(&mut w, j.bone_name.as_str());
        // lwAnimDataBone::write_matrix(, matrix)
      },
      None => {
        
      }
    }

  }

  fn write_node(node: joint, writer: &mut XmlWriter) {
    writer.start_element("node");
    lwAnimDataBone::write_ele_attrs(writer, node.bone_name.as_str());
  }

  pub fn write_ele_attrs(ele: &mut XmlWriter, name: &str) {
    ele.write_attribute("id", name);
    ele.write_attribute("sid", name);
    ele.write_attribute("name", name);
    ele.write_attribute("type", "JOINT");
  }

  pub fn write_matrix(ele: &mut XmlWriter, matrix: Matrix4<f32>) {
    ele.start_element("matrix");
    ele.write_attribute("sid", "transform");
    ele.write_text_fmt(format_args!("{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}", matrix[0][0], matrix[0][1], matrix[0][2], matrix[0][3], matrix[1][0], matrix[1][1], matrix[1][2], matrix[1][3], matrix[2][0], matrix[2][1], matrix[2][2], matrix[2][3], matrix[3][0], matrix[3][1], matrix[3][2], matrix[3][3]));
    ele.end_element();
  }
}