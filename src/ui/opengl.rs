use glium::*;

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
  position: [f32; 3],
}

impl Vertex {
  pub fn new(x: f32, y: f32, z: f32) -> Vertex {
    Vertex{
      position: [ x, y, z ]
    }
  }
}


pub fn get_vertex_shader<'a>() -> &'a str {
  r#"
  #version 140

  in vec3 position;

  void main() {
    gl_Position = vec4(position, 1.0);
  }

  "#
}

pub fn get_pixel_shader<'a>() -> &'a str {
  r#"
    #version 140

    out vec4 color;

    void main() {
        color = vec4(1.0, 0.0, 0.0, 1.0);
    }
"#
}


implement_vertex!(Vertex, position);