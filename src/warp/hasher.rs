use super::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct SaplingHasher {
    empty: Hash,
}

impl SaplingHasher {
    pub fn new() -> Self {
        let mut empty = [0u8; 32];
        empty[0] = 1;

        Self { empty }
    }
}

impl Default for SaplingHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for SaplingHasher {
    type D = Hash;
    fn empty(&self) -> Hash {
        self.empty
    }

    fn is_empty(&self, d: &Hash) -> bool {
        *d == self.empty
    }

    fn combine(&self, depth: u8, l: &Hash, r: &Hash, _check: bool) -> Hash {
        // println!("> {} {} {}", depth, hex::encode(l), hex::encode(r));
        crate::sapling::sapling_hash(depth, l, r)
    }

    fn parallel_combine(&self, depth: u8, layer: &[[u8; 32]], pairs: usize) -> Vec<Hash> {
        crate::sapling::sapling_parallel_hash(depth, layer, pairs)
    }
}
