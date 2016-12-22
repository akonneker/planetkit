use super::{ CellPos, Dir };

// TODO: take resolution, too.
pub fn advance(pos: &mut CellPos, dir: &mut Dir) {
    assert_eq!(&Dir::new(0), dir, "Ruh roh, this isn't actually implemented for reals yet.");

    // TODO: actual logic
    pos.x += 1;
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ Root, CellPos, Dir };

    #[test]
    fn advance_in_positive_x_direction() {
        let mut pos = CellPos {
            root: Root::new(0),
            x: 0,
            y: 0,
            z: 0,
        };
        let mut dir: Dir = 0.into();
        advance(&mut pos, &mut dir);
        assert_eq!(CellPos {
            root: Root::new(0),
            x: 1,
            y: 0,
            z: 0,
        }, pos);
        assert_eq!(Dir::new(0), dir);
    }
}