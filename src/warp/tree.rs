use super::bridge::{Bridge, CompactLayer};
use super::witness::Witness;
use super::{Hasher, Path, ReadWrite, DEPTH};
use anyhow::Result;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use std::fmt::Debug;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct MerkleTree<H: Hasher> {
    pub pos: usize,
    pub prev: [H::D; DEPTH + 1],
    pub witnesses: Vec<Witness<H>>,
    pub h: H,
}

impl<H: Hasher> MerkleTree<H> {
    pub fn empty(h: H) -> Self {
        MerkleTree {
            pos: 0,
            prev: std::array::from_fn(|_| h.empty()),
            witnesses: vec![],
            h,
        }
    }

    pub fn add_nodes(&mut self, height: u32, block_len: u32, nodes: &[(H::D, bool)]) -> Bridge<H> {
        // let ns: Vec<_> = nodes.iter().map(|n| n.0).collect();
        // println!("{ns:?}");
        // println!("self.pos {}", self.pos);
        assert!(!nodes.is_empty());
        let mut compact_layers = vec![];
        let mut new_witnesses = vec![];
        for (i, n) in nodes.iter().enumerate() {
            if n.1 {
                self.witnesses.push(Witness {
                    path: Path {
                        pos: self.pos + i,
                        value: n.0.clone(),
                        siblings: vec![],
                    },
                    fills: vec![],
                });
                new_witnesses.push(self.witnesses.len() - 1);
            }
        }
        log::debug!("{:?}", new_witnesses);

        let mut layer = vec![];
        let mut fill = self.h.empty();
        if !self.h.is_empty(&self.prev[0]) {
            layer.push(self.prev[0].clone());
            fill = nodes[0].0.clone();
        }
        layer.extend(nodes.iter().map(|n| n.0.clone()));

        for depth in 0..DEPTH {
            // println!("Merging {depth}");
            let mut new_fill = self.h.empty();
            let len = layer.len();
            let start = (self.pos >> depth) & 0xFFFF_FFFE;
            for &wi in new_witnesses.iter() {
                let w = &mut self.witnesses[wi];
                let i = (w.path.pos >> depth) - start;
                if i & 1 == 1 {
                    assert_ne!(layer[i - 1], self.h.empty());
                    w.path.siblings.push(layer[i - 1].clone());
                }
            }
            for w in self.witnesses.iter_mut() {
                if (w.path.pos >> depth) >= start {
                    let i = (w.path.pos >> depth) - start;
                    if i & 1 == 0 && i < len - 1 && !self.h.is_empty(&layer[i + 1]) {
                        w.fills.push(layer[i + 1].clone());
                    }
                }
            }
            log::debug!("w {:?}", self.witnesses);

            let pairs = (len + 1) / 2;
            let mut new_layer = vec![];
            if !self.h.is_empty(&self.prev[depth + 1]) {
                new_layer.push(self.prev[depth + 1].clone());
            }
            self.prev[depth] = self.h.empty();
            new_layer.extend_from_slice(&self.h.parallel_combine(depth as u8, &layer, pairs - 1));

            {
                let i = pairs - 1;
                let l = &layer[2 * i];
                if 2 * i + 1 < len {
                    if !self.h.is_empty(&layer[2 * i + 1]) {
                        let hn = self.h.combine(depth as u8, l, &layer[2 * i + 1], true);
                        new_layer.push(hn.clone());
                    } else {
                        new_layer.push(self.h.empty());
                        self.prev[depth] = l.clone();
                    }
                } else {
                    if !self.h.is_empty(l) {
                        self.prev[depth] = l.clone();
                    }
                    new_layer.push(self.h.empty());
                }
            }
            if new_layer.len() >= 2 && !self.h.is_empty(&new_layer[1]) {
                new_fill = new_layer[1].clone();
            }

            compact_layers.push(CompactLayer {
                prev: self.prev[depth].clone(),
                fill,
            });

            layer = new_layer;
            fill = new_fill;
            log::debug!("{layer:?}");
        }
        let pos = self.pos;
        self.pos += nodes.len();
        Bridge {
            height,
            block_len,
            pos,
            len: nodes.len(),
            layers: compact_layers.try_into().unwrap(),
        }
    }

    pub fn add_bridge(&mut self, bridge: &Bridge<H>) {
        for h in 0..DEPTH {
            if !self.h.is_empty(&bridge.layers[h].fill) {
                let s = self.pos >> (h + 1);
                for w in self.witnesses.iter_mut() {
                    let p = w.path.pos >> h;
                    if p & 1 == 0 && p >> 1 == s {
                        w.fills.push(bridge.layers[h].fill.clone());
                    }
                }
            }
            self.prev[h] = bridge.layers[h].prev.clone();
        }
        self.pos += bridge.len;
    }

    pub fn edge(&self, empty_roots: &[H::D]) -> [H::D; DEPTH] {
        let mut path = vec![];
        let mut h = self.h.empty();
        for depth in 0..DEPTH {
            let n = &self.prev[depth];
            if !self.h.is_empty(n) {
                h = self.h.combine(depth as u8, n, &h, false);
            } else {
                h = self.h.combine(depth as u8, &h, &empty_roots[depth], false);
            }
            path.push(h.clone());
        }
        path.try_into().unwrap()
    }

    pub fn add_witness(&mut self, w: Witness<H>) {
        self.witnesses.push(w);
    }
    pub fn remove_witness(&mut self, pos: usize) {
        self.witnesses.retain(|w| w.path.pos != pos);
    }

    pub fn write<W: Write>(&self, mut w: W) -> Result<()> {
        w.write_u64::<LE>(self.pos as u64)?;
        for p in self.prev.iter() {
            p.write(&mut w)?;
        }
        Ok(())
    }

    pub fn read<R: Read>(mut r: R, h: H) -> Result<Self> {
        let pos = r.read_u64::<LE>()? as usize;
        let mut prev = vec![];
        for _ in 0..DEPTH + 1 {
            let p = H::D::read(&mut r)?;
            prev.push(p);
        }
        Ok(Self {
            pos,
            prev: prev.try_into().unwrap(),
            witnesses: vec![],
            h,
        })
    }
}
