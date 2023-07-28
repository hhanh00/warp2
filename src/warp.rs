use anyhow::Result;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use std::fmt::{Debug, Formatter};
use std::io::{Read, Write};

pub use self::{bridge::Bridge, tree::MerkleTree, witness::Witness};

pub const DEPTH: usize = 32usize;
pub type Hash = [u8; 32];

pub trait ReadWrite {
    fn write<W: Write>(&self, w: W) -> Result<()>;
    fn read<R: Read>(r: R) -> Result<Self>
    where
        Self: Sized;
}

pub trait Hasher: Debug + Default {
    type D: Copy + Clone + PartialEq + Default + Debug + ReadWrite;
    fn empty(&self) -> Self::D;
    fn is_empty(&self, d: &Self::D) -> bool;
    fn combine(&self, depth: u8, l: &Self::D, r: &Self::D, check: bool) -> Self::D;
    fn parallel_combine(&self, depth: u8, layer: &[Self::D], pairs: usize) -> Vec<Self::D>;
}

impl ReadWrite for Hash {
    fn write<W: Write>(&self, mut w: W) -> Result<()> {
        w.write_all(self)?;
        Ok(())
    }

    fn read<R: Read>(mut r: R) -> Result<Self> {
        let mut h = [0u8; 32];
        r.read_exact(&mut h)?;
        Ok(h)
    }
}

pub mod bridge;
pub mod scan;
pub mod hasher;
pub mod tree;
pub mod witness;

pub struct Path<H: Hasher> {
    pub value: H::D,
    pub pos: usize,
    pub siblings: Vec<H::D>,
}

impl<H: Hasher> Debug for Path<H> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "value {:?}", self.value)?;
        write!(f, ", pos {}", self.pos)?;
        writeln!(f, ", siblings {:?}", self.siblings)
    }
}

impl<H: Hasher> Path<H> {
    pub fn empty(h: &H) -> Self {
        Path {
            value: h.empty(),
            pos: 0,
            siblings: vec![],
        }
    }

    fn write<W: Write>(&self, mut w: W) -> Result<()> {
        w.write_u64::<LE>(self.pos as u64)?;
        self.value.write(&mut w)?;
        w.write_u8(self.siblings.len() as u8)?;
        for s in self.siblings.iter() {
            s.write(&mut w)?;
        }
        Ok(())
    }

    fn read<R: Read>(mut r: R) -> Result<Self> {
        let pos = r.read_u64::<LE>()? as usize;
        let value = H::D::read(&mut r)?;
        let len = r.read_u8()? as usize;
        let mut siblings = vec![];
        for _ in 0..len {
            let s = H::D::read(&mut r)?;
            siblings.push(s);
        }
        Ok(Self {
            value,
            pos,
            siblings,
        })
    }
}

pub fn empty_roots<H: Hasher>(h: &H) -> [H::D; DEPTH] {
    let mut roots = vec![];
    roots.push(h.empty());
    for i in 0..DEPTH - 1 {
        roots.push(h.combine(i as u8, &roots[i], &roots[i], false));
    }
    roots.try_into().unwrap()
}
