// TODO: this belongs in the globe module root.
pub type IntCoord = u64;
pub type RootIndex = u8;

#[derive(Clone)]
pub struct Root {
    pub index: RootIndex,
}

pub struct CellPos {
    pub root: Root,
    pub x: IntCoord,
    pub y: IntCoord,
    // TODO: z
}

pub struct Cell {
    // For now we're just storing the elevation
    // at each X/Y position. That should all be cached
    // somewhere else, and this should have actual voxel data.
    pub height: f64,
}

// TODO: make this an actual voxmap.
pub struct Chunk {
    pub origin: CellPos,
    // Sorted by (z, y, x).
    pub cells: Vec<Cell>,
}
