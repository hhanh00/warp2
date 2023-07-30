use super::hasher::SaplingHasher;
use super::{Bridge, MerkleTree};
use anyhow::Result;
use byteorder::{ReadBytesExt, LE};
use prost::Message;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Read;
use std::marker::PhantomData;
use std::slice;
use std::sync::mpsc::channel;
use std::time::Instant;
use allo_isolate::IntoDart;
use zcash_client_backend::encoding::decode_extended_full_viewing_key;
use zcash_note_encryption::batch::try_compact_note_decryption;
use zcash_note_encryption::{EphemeralKeyBytes, ShieldedOutput};
use zcash_primitives::consensus::{BlockHeight, Network, Parameters};
use zcash_primitives::sapling::note_encryption::{PreparedIncomingViewingKey, SaplingDomain};
use zcash_primitives::sapling::Note;
use crate::lw_rpc::CompactBlock;

pub async fn full_scan(network: &Network, url: &str, fvk: &str, port: i64) -> Result<u64> {
    let zfvk =
        decode_extended_full_viewing_key(network.hrp_sapling_extended_full_viewing_key(), fvk)
            .unwrap();
    let nk = &zfvk.fvk.vk.nk;
    let ivk = zfvk.fvk.vk.ivk();
    let pivk = PreparedIncomingViewingKey::new(&ivk);
    let response = ureq::get(url).call()?;
    let mut reader = response.into_reader();
    let final_height = 2166554;
    let mut height;
    let mut tx_count = 0;
    let mut pos = 0;
    let mut cmtree = MerkleTree::empty(SaplingHasher::default());
    let mut nfs: HashMap<[u8; 32], (u32, u64)> = HashMap::new();
    let mut balance = 0i64;

    let start_time = Instant::now();

    let (tx_blocks, rx_blocks) = channel::<Vec<CompactBlock>>();
    std::thread::spawn(move || {
        let mut block_chunk = vec![];
        while let Ok(len) = reader.read_u32::<LE>() {
            let mut buf = vec![0; len as usize];
            reader.read_exact(&mut buf)?;
            let cb: CompactBlock = CompactBlock::decode(&*buf)?;
            let height = cb.height;
            tx_count += cb.vtx.len();
            block_chunk.push(cb);
            if tx_count > 100_000 || height == final_height {
                let blocks = block_chunk;
                block_chunk = vec![];
                tx_count = 0;
                tx_blocks.send(blocks)?;
            }
        }
        Ok::<_, anyhow::Error>(())
    });

    while let Ok(block_chunk) = rx_blocks.recv() {
        height = block_chunk[0].height;
        println!("\x1b[0;31mHeight: {height}\x1b[0m");
        unsafe {
            if let Some(post) = crate::api::POST_COBJ {
                post(port, &mut height.into_dart());
            }
        }
        let dec_block_chunk: Vec<_> = block_chunk
            .par_iter()
            .map(|b| decrypt_block(network, b, &pivk).unwrap())
            .collect();

        let mut notes = vec![];
        let mut bridges: Option<Bridge<SaplingHasher>> = None;
        let mut pos_start = pos;
        for db in dec_block_chunk.iter() {
            for n in db.notes.iter() {
                let note = &n.1;
                let p = pos + n.0;
                let nf = note.nf(nk, p as u64);
                let nv = note.value().inner();
                balance += nv as i64;
                nfs.insert(nf.0, (p, nv));
                notes.push((p, n.1.clone()));
            }
            pos += db.count_outputs;
        }

        let mut cmus: Vec<(super::Hash, bool)> = vec![];
        for (b, db) in block_chunk.iter().zip(dec_block_chunk.iter()) {
            // flush bridges or cmus (only one should exist)
            if let Some(bridge) = bridges.take() {
                // flush bridges
                cmtree.add_bridge(&bridge);
            }
            if !cmus.is_empty() {
                // flush nodes
                cmtree.add_nodes(0, 0, &cmus);
                cmus.clear();
            }
            assert_eq!(pos_start as usize, cmtree.pos);
            assert!(bridges.is_none());
            assert!(cmus.is_empty());

            if db.notes.is_empty() {
                // block has no new notes, use the block bridge
                if let Some(bridge) = b.sapling_bridge.as_ref() {
                    let bridge = Bridge::read(&*bridge.data, &SaplingHasher::default())?;
                    cmtree.add_bridge(&bridge);
                    pos_start += bridge.len as u32;
                    assert_eq!(pos_start as usize, cmtree.pos);
                }
            } else {
                for tx in b.vtx.iter() {
                    if let Some(sapling_bridge) = tx.sapling_bridge.as_ref() {
                        // tx was pruned
                        if !cmus.is_empty() {
                            // flush nodes
                            cmtree.add_nodes(0, 0, &cmus);
                            cmus.clear();
                        }

                        // accumulate bridge
                        let bridge =
                            Bridge::read(&*sapling_bridge.data, &SaplingHasher::default())?;
                        pos_start += bridge.len as u32;
                        bridges = match bridges.take() {
                            Some(mut b) => {
                                b.merge(&bridge, &cmtree.h);
                                Some(b)
                            }
                            None => Some(bridge),
                        };
                    } else {
                        if let Some(bridge) = bridges.take() {
                            // flush bridges
                            cmtree.add_bridge(&bridge);
                        }

                        // accumulate cmus
                        for o in tx.outputs.iter() {
                            cmus.push((o.cmu.clone().try_into().unwrap(), false));
                        }
                        pos_start += tx.outputs.len() as u32;
                        let cmus_pos_start = pos_start - cmus.len() as u32;
                        while !notes.is_empty() {
                            let n = &notes[0];
                            if (n.0 - cmus_pos_start) as usize >= cmus.len() {
                                break;
                            }
                            cmus[(n.0 - cmus_pos_start) as usize].1 = true;
                            notes.remove(0);
                        }
                    }
                }
            }
        }

        // flush bridges or cmus (only one should exist)
        if let Some(bridge) = bridges.take() {
            // flush bridges
            cmtree.add_bridge(&bridge);
        }
        if !cmus.is_empty() {
            // flush nodes
            cmtree.add_nodes(0, 0, &cmus);
            cmus.clear();
        }
        assert_eq!(pos_start as usize, cmtree.pos);

        // detect spends
        for b in block_chunk.iter() {
            for tx in b.vtx.iter() {
                for s in tx.spends.iter() {
                    if nfs.contains_key(&*s.nf) {
                        let (p, nv) = nfs[&*s.nf];
                        nfs.remove(&*s.nf);
                        cmtree.remove_witness(p as usize);
                        println!("Spent {nv}");
                        balance -= nv as i64;
                    }
                }
            }
        }
    }

    println!("Final height = {final_height}");
    let duration = start_time.elapsed();
    println!("Time elapsed in sapling full scan is: {:?}", duration);

    let er = super::empty_roots(&cmtree.h);
    let edge = cmtree.edge(&er);
    for w in cmtree.witnesses.iter() {
        let (root, _proof) = w.root(&er, &edge, &cmtree.h);
        println!("{} {}", w.path.pos, hex::encode(&root));
        // this is the anchor at the end height of the compact file
        assert_eq!(&root, &*hex::decode("44d4dce2ed4a15a775423e92802615cc5c3e4168f2dd2ca65ac2dd8d853bb523").unwrap());
    }

    for (i, (p, v)) in nfs.values().enumerate() {
        println!("Note #{i} / {p} = {v}");
    }
    println!("Balance = {balance}");

    // let mut client = connect_lightwalletd(url).await?;
    // let rep = client.get_tree_state(Request::new(BlockId { height: height as u64, hash: vec![] })).await?.into_inner();
    // let tree = hex::decode(&rep.sapling_tree).unwrap();
    // let tree = zcash_primitives::merkle_tree::CommitmentTree::<Node>::read(&*tree)?;
    // let root = tree.root();
    // println!("server root {}", hex::encode(&root.repr));

    Ok(balance as u64)
}

struct EncryptedOutput<P> {
    epk: [u8; 32],
    cmu: [u8; 32],
    enc: [u8; 52],
    _phantom: PhantomData<P>,
}

impl <P> EncryptedOutput<P> {
    pub fn new(co: crate::lw_rpc::CompactSaplingOutput) -> Self {
        Self {
            epk: co.epk.try_into().unwrap(),
            cmu: co.cmu.try_into().unwrap(),
            enc: co.ciphertext.try_into().unwrap(),
            _phantom: PhantomData::default(),
        }
    }
}

impl <P: Parameters> ShieldedOutput<SaplingDomain<P>, 52> for EncryptedOutput<P> {
    fn ephemeral_key(&self) -> EphemeralKeyBytes {
        EphemeralKeyBytes::from(self.epk)
    }

    fn cmstar_bytes(&self) -> [u8; 32] {
        self.cmu
    }

    fn enc_ciphertext(&self) -> &[u8; 52] {
        &self.enc
    }
}

struct DecBlock {
    count_outputs: u32,
    notes: Vec<(u32, Note)>,
}

fn decrypt_block(
    network: &Network,
    block: &crate::lw_rpc::CompactBlock,
    ivk: &PreparedIncomingViewingKey,
) -> Result<DecBlock> {
    let mut outputs = vec![];
    let mut pos = 0u32;
    for tx in block.vtx.iter() {
        for o in tx.outputs.iter() {
            let d = SaplingDomain::for_height(*network, BlockHeight::from_u32(block.height as u32));
            outputs.push((d, EncryptedOutput::new(o.clone())));
            pos += 1;
        }
        if let Some(sapling_bridge) = tx.sapling_bridge.as_ref() {
            pos += sapling_bridge.len;
        }
    }
    let decrypted =
        try_compact_note_decryption::<SaplingDomain<_>, EncryptedOutput<Network>>(slice::from_ref(ivk), &outputs);
    let mut notes = vec![];
    for (pos, dec) in decrypted.iter().enumerate() {
        if let Some(((note, _), _)) = dec {
            println!("Received {}", note.value().inner());
            notes.push((pos as u32, note.clone()));
        }
    }
    let block = DecBlock {
        count_outputs: pos,
        notes,
    };
    Ok(block)
}
