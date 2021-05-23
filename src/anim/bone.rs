use byteorder::{LittleEndian, ReadBytesExt};
use cgmath::{Matrix4, Quaternion, SquareMatrix, Transform, Vector3};
use chrono::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::{fs::File, io::Read, mem::size_of, u32};
use xmlwriter::*;
use collada::{document::ColladaDocument};

use crate::main;

use super::d3d::{lwMatrix43, lwMatrix44};

#[derive(Debug, PartialEq)]
enum BoneInfoKeyType {
    BoneKeyTypeMat43 = 1,
    BoneKeyTypeMat44,
    BoneKeyTypeQuaternion,
    BoneKeyTypeInvalid,
}
#[derive(Debug)]
struct BoneInfoHeader {
    bone_num: u32,
    frame_num: u32,
    dummy_num: u32,
    key_type: BoneInfoKeyType,
}
#[derive(Debug)]
struct BoneBaseInfo {
    name: [u8; 64],
    id: u32,
    parent_id: u32,
}

#[derive(PartialEq, Debug)]
struct BoneDummyInfo {
    id: u32,
    parent_bone_id: u32,
    mat: lwMatrix44,
}

#[derive(Debug, Clone)]
struct BoneKeyInfo {
    mat43_seq: Option<Vec<lwMatrix43>>,
    mat44_seq: Option<Vec<lwMatrix44>>,
    pos_seq: Option<Vec<Vector3<f32>>>,
    quat_seq: Option<Vec<Quaternion<f32>>>,
}

#[derive(Debug)]
pub struct AnimDataBone<'a> {
    header: BoneInfoHeader,
    base_seq: Vec<BoneBaseInfo>,
    dummy_seq: HashMap<u32, Vec<BoneDummyInfo>>,
    key_seq: Vec<BoneKeyInfo>,
    invmat_seq: Vec<lwMatrix44>,
    bone_map: HashMap<u32, RefCell<Joint<'a>>>,

    transformation_matrices: Vec<Vec<Matrix4<f32>>>,
    position_matrices: Vec<Matrix4<f32>>,
    root_joint: Option<&'a RefCell<Joint<'a>>>,
}

#[derive(Debug, Default)]
struct DummyObject {
    parent_id: u32,
    position_matrix: Option<Matrix4<f32>>,
    id: u32,
}

#[derive(Debug, Default)]
struct Joint<'a> {
    parent: Option<&'a RefCell<Joint<'a>>>,
    children: Vec<&'a RefCell<Joint<'a>>>,
    bone_id: u32,
    bone_name: String,
    parent_id: u32,
    transformation_matrix: Option<Matrix4<f32>>,
    position_matrix: Option<Matrix4<f32>>,
    dummies: Vec<DummyObject>,
}

impl BoneInfoHeader {
    pub fn new() -> BoneInfoHeader {
        BoneInfoHeader {
            bone_num: 0,
            frame_num: 0,
            dummy_num: 0,
            key_type: BoneInfoKeyType::BoneKeyTypeInvalid,
        }
    }
}

impl BoneBaseInfo {
    pub fn new() -> BoneBaseInfo {
        BoneBaseInfo {
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

impl BoneDummyInfo {
    pub fn new() -> BoneDummyInfo {
        BoneDummyInfo {
            id: 0,
            parent_bone_id: 0,
            mat: lwMatrix44::new([
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ]),
        }
    }
}

impl BoneKeyInfo {
    pub fn new() -> BoneKeyInfo {
        BoneKeyInfo {
            mat43_seq: None,
            mat44_seq: None,
            pos_seq: None,
            quat_seq: None,
        }
    }
}

impl<'a> AnimDataBone<'a> {
    pub fn new() -> AnimDataBone<'a> {
        AnimDataBone {
            header: BoneInfoHeader::new(),
            base_seq: Vec::new(),
            dummy_seq: HashMap::new(),
            key_seq: Vec::new(),
            invmat_seq: Vec::new(),
            position_matrices: Vec::new(),
            transformation_matrices: Vec::new(),
            bone_map: HashMap::new(),
            root_joint: None,
        }
    }

    pub fn load_from_file(&'a mut self, file: &mut File) -> String {
        // load all animation related data from the file
        self.load_header(file);
        self.load_base_seq(file);
        self.load_invmat_seq(file);
        self.load_dummy_seq(file);
        self.load_key_seq(file);

        // use loaded data to generate structures that can be consumed by animation programs (blender, maya etc)
        self.generate_position_matrices_at_rest();
        self.generate_transformation_matrices_for_all_frames();
        // generate a joint tree and write all the required data in collada format into a .dae file
        let xml_content = self.generate_joint_structure();
        xml_content
    }

    fn load_header(&mut self, file: &mut File) {
        file.seek(SeekFrom::Start(4)).unwrap();

        self.header.bone_num = file.read_u32::<LittleEndian>().unwrap();
        self.header.frame_num = file.read_u32::<LittleEndian>().unwrap();
        self.header.dummy_num = file.read_u32::<LittleEndian>().unwrap();
        let key_type = file.read_u32::<LittleEndian>().unwrap();

        self.header.key_type = match key_type {
            1 => BoneInfoKeyType::BoneKeyTypeMat43,
            2 => BoneInfoKeyType::BoneKeyTypeMat44,
            3 => BoneInfoKeyType::BoneKeyTypeQuaternion,
            _ => BoneInfoKeyType::BoneKeyTypeInvalid,
        };
    }

    fn load_base_seq(&mut self, file: &mut File) {
        for _ in 0..self.header.bone_num {
            let mut bone_seq = BoneBaseInfo::new();
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

    fn load_dummy_seq(&mut self, file: &mut File) -> Option<()> {
        for _ in 0..self.header.dummy_num {
            let mut bytes: [u8; 64] = [0; 64];
            let id = file.read_u32::<LittleEndian>().unwrap();
            let parent_bone_id = file.read_u32::<LittleEndian>().unwrap();
            file.read_exact(&mut bytes).unwrap();

            let decoded: [[f32; 4]; 4] = bincode::deserialize(&bytes).unwrap();
            let dummy_info = BoneDummyInfo {
                id,
                parent_bone_id,
                mat: lwMatrix44::new(decoded),
            };

            if self.dummy_seq.contains_key(&parent_bone_id) {
                let dummy = self.dummy_seq.get_mut(&parent_bone_id)?;
                dummy.push(dummy_info)
            } else {
                self.dummy_seq.insert(parent_bone_id, vec![dummy_info]);
            }
        }
        Some(())
    }

    fn load_key_seq(&mut self, file: &mut File) {
        let mut keys = vec![BoneKeyInfo::new(); self.header.bone_num as usize];

        match self.header.key_type {
            BoneInfoKeyType::BoneKeyTypeMat43 => {
                for i in 0..self.header.bone_num {
                    let key = &mut keys[i as usize];
                    let mut mat43_seq_vec = vec![
                        lwMatrix43 {
                            matrix: Default::default()
                        };
                        self.header.frame_num as usize
                    ];

                    let mut mat43_seq_bytes: Vec<u8> = vec![0; self.header.frame_num as usize];
                    file.read_exact(&mut mat43_seq_bytes).unwrap();

                    mat43_seq_vec[0] = bincode::deserialize(&mat43_seq_bytes).unwrap();
                    key.mat43_seq = Some(mat43_seq_vec);
                }
            }

            BoneInfoKeyType::BoneKeyTypeMat44 => {
                for i in 0..self.header.bone_num {
                    let key = &mut keys[i as usize];
                    let mut mat44_seq_vec =
                        vec![lwMatrix44::default(); self.header.frame_num as usize];

                    let mut mat44_seq_bytes: Vec<u8> = vec![0; self.header.frame_num as usize];
                    file.read_exact(&mut mat44_seq_bytes).unwrap();

                    let decoded: [[f32; 4]; 4] = bincode::deserialize(&mat44_seq_bytes).unwrap();
                    mat44_seq_vec[0] = lwMatrix44::new(decoded);
                    key.mat44_seq = Some(mat44_seq_vec);
                }
            }

            BoneInfoKeyType::BoneKeyTypeQuaternion => {
                for i in 0..self.header.bone_num {
                    let key = &mut keys[i as usize];
                    let mut pos_seq_vec =
                        vec![Vector3::new(0.0, 0.0, 0.0); self.header.frame_num as usize];

                    for j in 0..(self.header.frame_num as usize) {
                        let mut pos_seq_bytes: Vec<u8> = vec![0; size_of::<Vector3<f32>>()];
                        file.read_exact(&mut pos_seq_bytes).unwrap();
                        let deserialized: [f32; 3] = bincode::deserialize(&pos_seq_bytes).unwrap();
                        pos_seq_vec[j] =
                            Vector3::new(deserialized[0], deserialized[1], deserialized[2]);
                    }

                    key.pos_seq = Some(pos_seq_vec);

                    let mut quat_seq_vec =
                        vec![Quaternion::new(0.0, 0.0, 0.0, 0.0); self.header.frame_num as usize];

                    for j in 0..(self.header.frame_num as usize) {
                        let mut quat_seq_bytes: Vec<u8> = vec![0; size_of::<Quaternion<f32>>()];
                        file.read_exact(&mut quat_seq_bytes).unwrap();
                        let deserialized: [f32; 4] = bincode::deserialize(&quat_seq_bytes).unwrap();
                        quat_seq_vec[j] = Quaternion::new(
                            deserialized[3],
                            deserialized[0],
                            deserialized[1],
                            deserialized[2],
                        );
                    }

                    key.quat_seq = Some(quat_seq_vec);
                }
            }

            BoneInfoKeyType::BoneKeyTypeInvalid => {}
        };

        self.key_seq = keys;
    }

    pub fn get_num_bones(&self) -> usize {
        self.header.bone_num as usize
    }

    pub fn get_num_frames(&self) -> usize {
        self.header.frame_num as usize
    }

    /// goes through all the bones and generates a tree-like structure for the joints of the model
    ///                           | parent
    ///                          /\
    ///                       c1   c2
    /// so on and so forth.
    fn generate_joint_structure(&'a mut self) -> String {
        // create base map, containing all the joints of the skeleton, and as much data about them as is available
        for i in 0..self.get_num_bones() {
            let current_bone = &self.base_seq[i];
            if !self.bone_map.contains_key(&(i as u32)) {
                let position_matrix = Some(self.position_matrices[current_bone.id as usize]);
                let mut dummies: Vec<DummyObject> = Vec::new();

                if self.dummy_seq.contains_key(&current_bone.id) {
                    let dummy_objects = self.dummy_seq.get(&current_bone.id).unwrap();

                    for j in 0..dummy_objects.len() {
                        dummies.push(DummyObject {
                            id: dummy_objects[j].id,
                            parent_id: dummy_objects[j].parent_bone_id,
                            position_matrix: Some(dummy_objects[j].mat.matrix),
                        });
                    }
                }

                self.bone_map.insert(
                    i as u32,
                    RefCell::new(Joint {
                        bone_id: current_bone.id,
                        parent: None,
                        bone_name: current_bone.get_name(),
                        children: Vec::new(),
                        parent_id: current_bone.parent_id,
                        position_matrix,
                        dummies,
                        ..Default::default()
                    }),
                );
            }
        }

        // link all the joints to their parent/children joints
        for (_, v) in self.bone_map.iter() {
            let mut current_joint = v.borrow_mut();
            if current_joint.parent_id != u32::MAX {
                if self.bone_map.contains_key(&current_joint.parent_id) {
                    let parent_bone = self.bone_map.get(&current_joint.parent_id).unwrap();
                    let mut parent_joint = parent_bone.borrow_mut();

                    current_joint.parent = Some(parent_bone);
                    parent_joint.children.push(v);
                } else {
                    println!(
                        "Parent bone not found in map. Parent Bone ID - {}, Current Bone ID - {}",
                        current_joint.parent_id, current_joint.bone_id
                    );
                    panic!();
                }
            }
        }

        let root_joint = self.bone_map.get(&0).unwrap();
        self.root_joint = Some(root_joint);

        self.write_collada_data()
    }

    fn generate_position_matrices_at_rest(&mut self) {
        let mut position_matrices: Vec<Matrix4<f32>> = Vec::new();

        for i in 0..self.header.bone_num as usize {
            let key = &self.key_seq[i];
            // let current_bone_name
            
            let mut current_matrix: Matrix4<f32> = SquareMatrix::identity();
            
            match &self.header.key_type {
                BoneInfoKeyType::BoneKeyTypeQuaternion => {
                    let frame_quat = match &key.quat_seq {
                        Some(e) => e,
                        None => {
                            panic!("No frame_quat found");
                        }
                    };
                    let frame_pos = match &key.pos_seq {
                        Some(e) => e,
                        None => {
                            panic!("No frame_pos found");
                        }
                    };
                    let quat: Quaternion<f32> = Quaternion::from(frame_quat[0]);
                    let offset: Vector3<f32> = Vector3::from(frame_pos[0]);
                    current_matrix = Matrix4::from(quat) * Matrix4::from_translation(offset);
                },

                BoneInfoKeyType::BoneKeyTypeMat43 => {
                    let mat43 = match &key.mat43_seq  {
                        Some(vec) => vec,
                        None => panic!("No mat43 seq found for bone")
                    };
    
                    current_matrix = mat43[0].get_matrix4(); 
                },
    
                BoneInfoKeyType::BoneKeyTypeMat44 => {
                    let mat44 = match &key.mat44_seq {
                        Some(vec) => vec,
                        None => panic!("No mat44 seq found for bone"),
                    };
    
                    current_matrix = mat44[0].matrix;
                }

                _ => {}
            }

            position_matrices.push(current_matrix);
        }

        self.position_matrices = position_matrices;
    }

    fn generate_transformation_matrices_for_all_frames(&mut self) {
      for i in 0..self.get_num_bones() {
        let mut finish_matrices: Vec<Matrix4<f32>> = Vec::new();
        let key = &self.key_seq[i];

        
        let mut current_matrix: Matrix4<f32> = SquareMatrix::identity();
        for j in 0..self.get_num_frames() {
            match &self.header.key_type {
                BoneInfoKeyType::BoneKeyTypeQuaternion => {
                let frame_quat = match &key.quat_seq {
                  Some(e) => e,
                  None => {
                      panic!("No frame_quat found");
                  }
                };
                let frame_pos = match &key.pos_seq {
                    Some(e) => e,
                    None => {
                        panic!("No frame_pos found");
                    }
                };
                let quat: Quaternion<f32> = Quaternion::from(frame_quat[j]);
                let offset: Vector3<f32> = Vector3::from(frame_pos[j]);
                current_matrix = Matrix4::from(quat) * Matrix4::from_translation(offset);
            },

            BoneInfoKeyType::BoneKeyTypeMat43 => {
                let mat43 = match &key.mat43_seq  {
                    Some(vec) => vec,
                    None => panic!("No mat43 seq found for bone")
                };

                current_matrix = mat43[j].get_matrix4(); 
            },

            BoneInfoKeyType::BoneKeyTypeMat44 => {
                let mat44 = match &key.mat44_seq {
                    Some(vec) => vec,
                    None => panic!("No mat44 seq found for bone"),
                };

                current_matrix = mat44[j].matrix;
            }

            _ => {}
          }

          finish_matrices.push(current_matrix);
        }

        self.transformation_matrices.push(finish_matrices);
      }
    }
    
    pub fn get_transforms_for_frame(&self, frame: usize) -> Vec<Matrix4<f32>> {
        let mut finish_matrices: Vec<Matrix4<f32>> = Vec::new();

        for i in 0..self.header.bone_num as usize {
            let key = &self.key_seq[i];
            let current_bone_name = &self.base_seq[i].get_name();
            let frame_quat = match &key.quat_seq {
                Some(e) => e,
                None => {
                    panic!("No frame_quat found");
                }
            };
            let frame_pos = match &key.pos_seq {
                Some(e) => e,
                None => {
                    panic!("No frame_pos found");
                }
            };
            let mut current_matrix: Matrix4<f32> = SquareMatrix::identity();

            match &self.header.key_type {
                BoneInfoKeyType::BoneKeyTypeQuaternion => {
                    let quat: Quaternion<f32> = Quaternion::from(frame_quat[frame]);
                    let offset: Vector3<f32> = Vector3::from(frame_pos[frame]);
                    let mat1 = Matrix4::from(quat);
                    let mat2 = Matrix4::from_translation(offset);
                    current_matrix = Matrix4::from(quat) * Matrix4::from_translation(offset);
                }

                _ => {}
            }

            finish_matrices.push(current_matrix);
        }

        finish_matrices
    }

    pub fn get_transforms_for_frame_and_bone(
        &self,
        frame: usize,
        bone: usize,
    ) -> Vec<Matrix4<f32>> {
        let mut finish_matrices: Vec<Matrix4<f32>> = Vec::new();
        let key = &self.key_seq[bone];
        let frame_quat = match &key.quat_seq {
            Some(e) => e,
            None => {
                panic!("No frame_quat found");
            }
        };
        let frame_pos = match &key.pos_seq {
            Some(e) => e,
            None => {
                panic!("No frame_pos found");
            }
        };
        let mut current_matrix: Matrix4<f32> = SquareMatrix::identity();

        match &self.header.key_type {
            BoneInfoKeyType::BoneKeyTypeQuaternion => {
                let quat: Quaternion<f32> = Quaternion::from(frame_quat[frame]);
                let offset: Vector3<f32> = Vector3::from(frame_pos[frame]);
                let mat1 = Matrix4::from(quat);
                let mat2 = Matrix4::from_translation(offset);
                current_matrix = Matrix4::from(quat) * Matrix4::from_translation(offset);
            }

            _ => {}
        }

        finish_matrices.push(current_matrix);

        finish_matrices
    }

    pub fn apply_transforms(&self, transforms: Vec<Matrix4<f32>>) -> Vec<Vector3<f32>> {
        let mut positions: Vec<Vector3<f32>> = Vec::with_capacity(self.header.bone_num as usize);

        for i in 0..self.header.bone_num as usize {
            let inv_mat = Matrix4::from(self.invmat_seq[i].matrix);
            let original_mat = inv_mat.invert().unwrap();
            let start_pos =
                Vector3::new(original_mat[0][3], original_mat[1][3], original_mat[2][3]);
            let transformation_matrix = inv_mat * transforms[i];

            // if i >= 8 {
            //   println!("{:?}", transformation_matrix);
            // }
            positions.push(transformation_matrix.transform_vector(start_pos));
        }

        positions
    }

    fn write_collada_data(&self) -> String {
        let options = Options {
            use_single_quote: false,
            ..Default::default()
        };

        let mut writer = XmlWriter::new(options);
        writer.start_element("COLLADA");
        writer.write_attribute("xmlns", "http://www.collada.org/2005/11/COLLADASchema");
        writer.write_attribute("version", "1.4.1");

        self.write_asset_data(&mut writer);
        self.write_visual_scene_data(&mut writer);
        self.write_animation_data(&mut writer);
        self.write_scene_element(&mut writer);

        writer.end_element();

        let content = writer.end_document();
        content
    }

    fn write_asset_data(&self, writer: &mut XmlWriter) {
        // asset tag
        writer.start_element("asset");

        writer.start_element("contributor");
        writer.start_element("author");
        writer.write_text("Perseus");
        writer.end_element();
        writer.end_element();

        // created-at tag
        writer.start_element("created");
        writer.write_text(&chrono::Utc::now().to_string());
        writer.end_element();

        // up-axis
        writer.start_element("up_axis");
        writer.write_text("Z_UP");
        writer.end_element();
        
        // end asset
        writer.end_element();
    }

    fn write_visual_scene_data(&self, writer: &mut XmlWriter) {
        writer.start_element("library_visual_scenes");

        // start the visual scene tag
        writer.start_element("visual_scene");
        writer.write_attribute("id", "Scene");
        writer.write_attribute("name", "Scene");

        // base skeleton node
        writer.start_element("node");
        writer.write_attribute("id", "Skeleton");
        writer.write_attribute("name", "Skeleton");
        writer.write_attribute("type", "NODE");

        self.write_joint_node(writer, self.root_joint.unwrap());

        writer.end_element();
        writer.end_element();

        writer.end_element();
    }

    fn write_joint_node(&self, writer: &mut XmlWriter, joint: &RefCell<Joint>) {
        let joint_data = joint.borrow();

        writer.start_element("node");
        writer.write_attribute("id", &joint_data.bone_name.replace(" ", "_"));
        writer.write_attribute("sid", &joint_data.bone_name.replace(" ", "_"));
        writer.write_attribute("name", &joint_data.bone_name);
        writer.write_attribute("type", "JOINT");

        self.write_matrix(writer, joint_data.position_matrix.unwrap());

        if joint_data.dummies.len() > 0 {
            for i in 0..joint_data.dummies.len() {
                self.write_dummy_node(writer, &joint_data.dummies[i]);
            }
        }

        for i in 0..joint_data.children.len() {
            self.write_joint_node(writer, joint_data.children[i]);
        }

        writer.end_element();
    }

    fn write_dummy_node(&self, writer: &mut XmlWriter, dummy: &DummyObject) {
        writer.start_element("node");
        writer.write_attribute("id", &format!("Dummy_{}", dummy.id));
        writer.write_attribute("name", &format!("Dummy {}", dummy.id));
        writer.write_attribute("type", "NODE");
        self.write_matrix(writer, dummy.position_matrix.unwrap());

        writer.end_element();
    }

    fn write_animation_data(&self, writer: &mut XmlWriter) {
      writer.start_element("library_animations");

      for i in 0..self.get_num_bones() {
        self.write_animation_element(writer, i);
      }

      writer.end_element();
    }

    fn write_animation_element(&self, writer: &mut XmlWriter, bone_index: usize) {
      let bone_data = &self.base_seq[bone_index];
      let sanitized_bone_name = bone_data.get_name().replace(" ", "_");

      writer.start_element("animation");
      writer.write_attribute("id", &format!("{}_pose_matrix", sanitized_bone_name));
      writer.write_attribute("name", &format!("{}_pose_matrix", sanitized_bone_name));
    
      writer.start_element("source");
      writer.write_attribute("id", &format!("{}_pose_matrix-input", sanitized_bone_name));

      writer.start_element("float_array");
      writer.write_attribute("id", &format!("{}_pose_matrix-input-array",sanitized_bone_name)); 
      writer.write_attribute("count", &self.header.frame_num);
      for i in 0..self.header.frame_num as usize {
        writer.write_text(&(i as f32 / 25.00).to_string());
      }
      writer.end_element();

      writer.start_element("technique_common");
      writer.start_element("accessor");
      writer.write_attribute("source", &format!("#{}_pose_matrix-input-array", sanitized_bone_name));
      writer.write_attribute("count", &self.header.frame_num);
      writer.write_attribute("stride", &1);
      writer.start_element("param");
      writer.write_attribute("name", "TIME");
      writer.write_attribute("type", "float");
      writer.end_element();
      writer.end_element();
      writer.end_element();
      writer.end_element();

      writer.start_element("source");
      writer.write_attribute("id", &format!("{}_pose_matrix-output", sanitized_bone_name));
      writer.start_element("float_array");
      writer.write_attribute("id", &format!("{}_pose_matrix-output-array", sanitized_bone_name));
      writer.write_attribute("count", &(16 * self.header.frame_num));
      AnimDataBone::write_all_matrices(writer, &self.transformation_matrices[bone_index]);
      writer.end_element();
      writer.start_element("technique_common");
      writer.start_element("accessor");
      writer.write_attribute("source", &format!("#{}_pose_matrix-output-array", sanitized_bone_name));
      writer.write_attribute("count", &self.header.frame_num);
      writer.write_attribute("stride", &16);
      writer.start_element("param");
      writer.write_attribute("name", "TRANSFORM");
      writer.write_attribute("type", "float4x4");
      writer.end_element();
      writer.end_element();
      writer.end_element();
      writer.end_element();

      writer.start_element("source");
      writer.write_attribute("id", &format!("{}_pose_matrix-interpolation", sanitized_bone_name));
      writer.start_element("Name_array");
      writer.write_attribute("id", &format!("{}_pose_matrix-interpolation-array", sanitized_bone_name));
      writer.write_attribute("count", &self.header.frame_num);
      for _ in 0..self.header.frame_num {
        writer.write_text("LINEAR");
      }
      writer.end_element();
      writer.start_element("technique_common");
      writer.start_element("accessor");
      writer.write_attribute("source", &format!("#{}_pose_matrix-interpolation-array", sanitized_bone_name));
      writer.write_attribute("count", &self.header.frame_num);
      writer.write_attribute("stride", &1);
      writer.start_element("param");
      writer.write_attribute("name", "INTERPOLATION");
      writer.write_attribute("type", "name");
      writer.end_element();
      writer.end_element();
      writer.end_element();
      writer.end_element();

      writer.start_element("sampler");
      writer.write_attribute("id", &format!("{}_pose_matrix-sampler", sanitized_bone_name));
      writer.start_element("input");
      writer.write_attribute("semantic", "INPUT");
      writer.write_attribute("source", &format!("#{}_pose_matrix-input", sanitized_bone_name));
      writer.end_element();
      writer.start_element("input");
      writer.write_attribute("semantic", "OUTPUT");
      writer.write_attribute("source", &format!("#{}_pose_matrix-output", sanitized_bone_name));
      writer.end_element();
      writer.start_element("input");
      writer.write_attribute("semantic", "INTERPOLATION");
      writer.write_attribute("source", &format!("#{}_pose_matrix-interpolation", sanitized_bone_name));
      writer.end_element();
      writer.end_element();
      
      writer.start_element("channel");
      writer.write_attribute("source", &format!("#{}_pose_matrix-sampler", sanitized_bone_name));
      writer.write_attribute("target", &format!("{}/transform", sanitized_bone_name));
      writer.end_element();

      writer.end_element();
    }

    fn write_scene_element(&self, writer: &mut XmlWriter) {
        writer.start_element("scene");
        writer.start_element("instance_visual_scene");
        writer.write_attribute("url", "#Scene");
        writer.end_element();
        writer.end_element();
    }

    pub fn write_matrix(&self, ele: &mut XmlWriter, matrix: Matrix4<f32>) {
        ele.start_element("matrix");
        ele.write_attribute("sid", "transform");
        ele.write_text_fmt(format_args!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} ",
            matrix[0][0],
            matrix[0][1],
            matrix[0][2],
            matrix[0][3],
            matrix[1][0],
            matrix[1][1],
            matrix[1][2],
            matrix[1][3],
            matrix[2][0],
            matrix[2][1],
            matrix[2][2],
            matrix[2][3],
            matrix[3][0],
            matrix[3][1],
            matrix[3][2],
            matrix[3][3]
        ));
        ele.end_element();
    }

    pub fn write_all_matrices(ele: &mut XmlWriter, matrix: &Vec<Matrix4<f32>>) {
      for i in 0..matrix.len() {
        let mat = matrix[i];
        ele.write_text_fmt(format_args!("{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}", mat[0][0], mat[0][1], mat[0][2], mat[0][3], mat[1][0], mat[1][1], mat[1][2], mat[1][3], mat[2][0], mat[2][1], mat[2][2], mat[2][3], mat[3][0], mat[3][1], mat[3][2], mat[3][3]));
      }
    }


    pub fn load_data_from_collada_skeleton(&mut self, doc: &ColladaDocument) {
        let skeletons = doc.get_skeletons().unwrap();

        // we support only one skeleton in an animation for ToP lab files
        if skeletons.len() > 1 {
            panic!("More than one skeleton found. Invalid file format.");
        }
    
        let main_skeleton = &skeletons[0];
        
        println!("{:?}", main_skeleton.joints);
        // println!("{:?}", doc.get_bind_data_set().unwrap().bind_data[0]);
    }









}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_loads_header_info_correctly() {
        let mut bone = AnimDataBone::new();
        let path = Path::new("./src/tests/anim-quat.lab");
        let display = path.display();

        let mut file = match File::open(&path) {
            Err(why) => panic!("Couldn\'t open {}: {}", display, why),
            Ok(file) => file,
        };

        bone.load_header(&mut file);
        assert_eq!(bone.header.bone_num, 35);
        assert_eq!(bone.header.frame_num, 228);
        assert_eq!(bone.header.dummy_num, 2);
        assert_eq!(bone.header.key_type, BoneInfoKeyType::BoneKeyTypeQuaternion);
    }

    #[test]
    fn it_loads_base_bone_info_correctly() {
        let mut bone = AnimDataBone::new();
        let path = Path::new("./src/tests/anim-quat.lab");
        let display = path.display();

        let mut file = match File::open(&path) {
            Err(why) => panic!("Couldn\'t open {}: {}", display, why),
            Ok(file) => file,
        };

        bone.load_header(&mut file);
        bone.load_base_seq(&mut file);

        struct BoneBaseTestInfo {
            pub id: u32,
            pub parent_id: u32,
            pub name: String,
        }

        let actual_bones = vec![
            BoneBaseTestInfo {
                id: 0,
                parent_id: 4294967295,
                name: String::from("Bip01"),
            },
            BoneBaseTestInfo {
                id: 1,
                parent_id: 0,
                name: String::from("Bip01 Footsteps"),
            },
            BoneBaseTestInfo {
                id: 2,
                parent_id: 0,
                name: String::from("Bip01 Pelvis"),
            },
            BoneBaseTestInfo {
                id: 20,
                parent_id: 19,
                name: String::from("Bip01 R Finger0Nub"),
            },
            BoneBaseTestInfo {
                id: 33,
                parent_id: 32,
                name: String::from("Bip01 Tail2"),
            },
        ];

        // test some random bones against the base seq
        for i in 0..actual_bones.len() {
            let bone_id = actual_bones[i].id as usize;
            assert_eq!(actual_bones[i].id, bone.base_seq[bone_id].id);
            assert_eq!(actual_bones[i].parent_id, bone.base_seq[bone_id].parent_id);
            assert_eq!(actual_bones[i].name, bone.base_seq[bone_id].get_name());
        }
    }
}
