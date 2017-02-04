use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use gfx;
use obj::{ mtl, obj };

use super::{ MeshRepository, MeshHandle, Vertex };

pub const GRAY: [f32; 3] = [ 0.5, 0.5, 0.5 ];

pub fn make_obj_mesh<
    OP: AsRef<Path>,
    MP: AsRef<Path>,
    R: gfx::Resources,
    F: gfx::Factory<R>,
>(
    obj_path: OP,
    mtl_path: MP,
    scale: f32,
    factory: &mut F,
    mesh_repo: &mut MeshRepository<R>,
) -> MeshHandle {
    let mut vertex_data = Vec::<Vertex>::new();
    let mut index_vec = Vec::<u32>::new();

    let mut obj_file = File::open(obj_path).expect("Failed to open obj file");
    let mut obj_str = String::new();
    obj_file.read_to_string(&mut obj_str).expect("Failed to read obj file");

    let mut mtl_file = File::open(mtl_path).expect("Failed to open mtl file");
    let mut mtl_str = String::new();
    mtl_file.read_to_string(&mut mtl_str).expect("Failed to read mtl file");

    let obj_set = obj::parse(obj_str).unwrap();
    let mtl_set = mtl::parse(mtl_str).unwrap();
    let mtl_map: HashMap<_, _> = mtl_set.materials.iter().map(|m| (m.name.as_str(), m)).collect();
    for object in &obj_set.objects {
        add_object(object, &mtl_map, scale, &mut vertex_data, &mut index_vec);
    }

    mesh_repo.create(factory, vertex_data, index_vec)
}

fn add_object(object: &obj::Object, mtl_map: &HashMap<&str, &mtl::Material>, scale: f32,
              vertex_data: &mut Vec<Vertex>, index_vec: &mut Vec<u32>) {
    for v in &object.vertices {
        vertex_data.push(Vertex::new([v.x as f32 * scale, v.y as f32 * scale, v.z as f32 * scale],
                                     GRAY));
    }
    for g in &object.geometry {
        for shape in &g.shapes {
            match shape.primitive {
                obj::Primitive::Triangle(i, j, k) => {
                    let (i_vi, _, _) = i;
                    let (j_vi, _, _) = j;
                    let (k_vi, _, _) = k;

                    if let Some(ref material) = g.material_name {
                        let ref material = mtl_map[material.as_str()];
                        let a_color = [material.color_diffuse.r as f32,
                                       material.color_diffuse.g as f32,
                                       material.color_diffuse.b as f32];
                        vertex_data[i_vi].a_color = a_color;
                        vertex_data[j_vi].a_color = a_color;
                        vertex_data[k_vi].a_color = a_color;
                    }

                    index_vec.push(i_vi as u32);
                    index_vec.push(j_vi as u32);
                    index_vec.push(k_vi as u32);
                },
                _ => { println!("WARNING: Skipping unsupported obj primitive"); },
            }
        }
    }
}
