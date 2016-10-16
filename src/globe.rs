use noise;
use icosahedron;

use chunk::{ IntCoord, Chunk, Root, CellPos, Cell };

const ROOT_QUADS: u8 = 10;

pub struct Spec {
    seed: u32,
    radius: f64,
    // These are the full width/height of a given root quad or chunk's voxmap;
    // i.e. not an exponent.
    root_resolution: IntCoord,
    chunk_resolution: IntCoord,
}

impl Spec {
    pub fn is_valid(&self) -> bool {
        // Chunk resolution needs to be a factor of root resolution.
        let chunks_per_root_side = self.root_resolution / self.chunk_resolution;
        chunks_per_root_side * self.chunk_resolution == self.root_resolution
    }
}

// TODO: split out a WorldGen type that handles all the procedural
// generation, because none of that really needs to be tangled
// with the realised Globe.
pub struct Globe {
    spec: Spec,
    // Permutation table for noise
    pt: noise::Seed,
    // TODO: figure out what structure to store these in.
    // You'll never have all chunks loaded in the real world.
    //
    // TODO: you'll probably also want to store some lower-res
    // pseudo-chunks for rendering planets at a distance.
    // But maybe you can put that off?
    chunks: Vec<Chunk>,
}

impl Globe {
    pub fn new(spec: Spec) -> Globe {
        assert!(spec.is_valid(), "Invalid globe spec!");
        let pt = noise::Seed::new(spec.seed);
        let mut globe = Globe {
            spec: spec,
            pt: pt,
            chunks: Vec::new(),
        };
        globe.build_all_chunks();
        globe
    }

    pub fn new_example() -> Globe {
        Globe::new(
            Spec {
                seed: 12,
                radius: 1.0, // TODO: make it ~Earth
                root_resolution: 16,
                chunk_resolution: 4,
            }
        )
    }

    pub fn build_all_chunks(&mut self) {
        let chunks_per_root_side = self.spec.root_resolution / self.spec.chunk_resolution;
        for root_index in 0..ROOT_QUADS {
            let root = Root { index: root_index };
            for y in 0..chunks_per_root_side {
                for x in 0..chunks_per_root_side {
                    let origin = CellPos {
                        root: root.clone(),
                        x: x * self.spec.chunk_resolution,
                        y: y * self.spec.chunk_resolution,
                    };
                    self.build_chunk(origin);
                }
            }
        }
    }

    pub fn build_chunk(&mut self, origin: CellPos) {
        // TODO: get parameters from spec
        let noise = noise::Brownian3::new(noise::perlin3, 4).wavelength(1.0);
        let mut cells: Vec<Cell> = Vec::new();
        let end_x = origin.x + self.spec.chunk_resolution;
        let end_y = origin.y + self.spec.chunk_resolution;
        for cell_y in origin.y..end_y {
            for cell_x in origin.x..end_x {
                // Calculate height for this cell from world spec.

                // TODO: project onto the globe!
                let (x, y, z) = (1.0, 2.0, 3.0);

                // Vary a little bit around 1.0.
                let height = noise.apply(&self.pt, &[x, y, z]) * 0.1 + 1.0;
                cells.push(Cell {
                    height: height,
                });
            }
        }
    }

    // Make vertices and list of indices into that array for triangle faces.
    pub fn make_geometry(&self) -> (Vec<::Vertex>, Vec<u16>) {
        // Massage vertex data into form that gfx wants.
        // Set up some noise so we can mutate the base
        // icosahedron vertices.
        let noise = noise::Brownian3::new(noise::perlin3, 4).wavelength(1.0);
        let mutated_vertices: Vec<::Vertex> = icosahedron::VERTICES
            .iter()
            .map(|v| {
                let (x, y, z) = (
                    v[0] * self.spec.radius,
                    v[1] * self.spec.radius,
                    v[2] * self.spec.radius,
                );
                // Vary a little bit around 1.0.
                let val = noise.apply(&self.pt, &[x, y, z]) * 0.1 + 1.0;
                // Set the color later when we clone it.
                // TODO: This code is a mess. Hack first, clean it up later.
                ::Vertex::new([
                    (x * val) as f32,
                    (y * val) as f32,
                    (z * val) as f32,
                ], [0.0, 0.0, 0.0])
            })
            .collect();

        // Give each face its own unique copies of the vertices,
        // so that we can colour the faces independently.
        let mut vertex_data: Vec<::Vertex> = Vec::new();
        let mut face_index: usize = 0;
        let index_vec: Vec<u16> = icosahedron::FACES
            .iter()
            .flat_map(|f| {
                let first_vertex_index = vertex_data.len();
                for v in 0..3 {
                    let mut colored_vertex = mutated_vertices[f[v]].clone();
                    // Give adjacent triangles the same colour.
                    // We want to highlight the connection between
                    // the two triangles that make up each quad.
                    colored_vertex.a_color = icosahedron::RAINBOW[face_index / 2];
                    vertex_data.push(colored_vertex);
                }
                face_index += 1;
                vec![
                    first_vertex_index,
                    first_vertex_index + 1,
                    first_vertex_index + 2,
                ]
            })
            .map(|vi| vi as u16)
            .collect();

        (vertex_data, index_vec)
    }
}