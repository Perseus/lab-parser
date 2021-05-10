use serde::{ Serialize, Deserialize };
use cgmath::*;

#[derive(PartialEq, Debug, Clone)]
pub struct lwMatrix44 {
  pub matrix: Matrix4<f32>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct lwMatrix43 {
  pub matrix: [[f32; 3]; 4],
}

impl lwMatrix43 {
  pub fn get_matrix4(&self) -> Matrix4<f32> {
    Matrix4::new(
      self.matrix[0][0], self.matrix[1][0], self.matrix[2][0], self.matrix[3][0],
      self.matrix[0][1], self.matrix[1][1], self.matrix[2][1], self.matrix[3][1],
      self.matrix[0][2], self.matrix[1][2], self.matrix[2][2], self.matrix[3][2],
      0.0, 0.0, 0.0, 1.0,
    )
  }
}

impl lwMatrix44 {
  pub fn new(mat: [[f32; 4]; 4]) -> lwMatrix44 {
    lwMatrix44{
      matrix: Matrix4::new(mat[0][0], mat[1][0], mat[2][0], mat[3][0], mat[0][1], mat[1][1], mat[2][1], mat[3][1], 
        mat[0][2], mat[1][2], mat[2][2], mat[3][2], mat[0][3], mat[1][3], mat[2][3], mat[3][3] ),
    }
  }

  pub fn default() -> lwMatrix44 {
    lwMatrix44::new([[0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]])
  }
}