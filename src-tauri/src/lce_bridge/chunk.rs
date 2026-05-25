use crate::lce_bridge::registry;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

const SECTIONS: usize = 24;
const BLOCKS_PER_SECTION: usize = 4096;
pub const FULL_CHUNK_SIZE: usize = SECTIONS * BLOCKS_PER_SECTION;

pub struct LceChunkBuilder {
    blocks: Vec<u16>,
    block_light: Vec<u8>,
    sky_light: Vec<u8>,
    biomes: Vec<u8>,
}

impl LceChunkBuilder {
    pub fn new() -> Self {
        Self {
            blocks: vec![0u16; FULL_CHUNK_SIZE],
            block_light: vec![0u8; FULL_CHUNK_SIZE / 2],
            sky_light: vec![0u8; FULL_CHUNK_SIZE / 2],
            biomes: vec![0u8; 256],
        }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: u16) {
        if y >= SECTIONS * 16 { return; }
        let seci = y / 16;
        let lcy = y % 16;
        let idx = seci * BLOCKS_PER_SECTION + (x * 16 + z) * 16 + lcy;
        if idx < self.blocks.len() {
            self.blocks[idx] = block;
        }
    }

    pub fn set_sky_light(&mut self, x: usize, y: usize, z: usize, light: u8) {
        if y >= SECTIONS * 16 { return; }
        let idx = (y / 16) * 2048 + (x * 16 + z) * 16 + (y % 16);
        if idx < self.sky_light.len() {
            let nibble_idx = idx / 2;
            if idx % 2 == 0 {
                self.sky_light[nibble_idx] = (self.sky_light[nibble_idx] & 0xF0) | (light & 0x0F);
            } else {
                self.sky_light[nibble_idx] = (self.sky_light[nibble_idx] & 0x0F) | ((light & 0x0F) << 4);
            }
        }
    }

    pub fn set_biome(&mut self, x: usize, z: usize, biome: u8) {
        let idx = z * 16 + x;
        if idx < self.biomes.len() {
            self.biomes[idx] = biome;
        }
    }

    pub fn build_raw_data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(
            self.blocks.len() * 2 + self.sky_light.len() + self.block_light.len() + self.biomes.len()
        );
        for &b in &self.blocks {
            data.extend_from_slice(&b.to_be_bytes());
        }
        data.extend_from_slice(&self.sky_light);
        data.extend_from_slice(&self.block_light);
        data.extend_from_slice(&self.biomes);
        data
    }

    pub fn snapshot(&self) -> Self {
        Self {
            blocks: self.blocks.clone(),
            block_light: self.block_light.clone(),
            sky_light: self.sky_light.clone(),
            biomes: self.biomes.clone(),
        }
    }

    pub fn pack_half_height_nibbles(&self) -> (Vec<u8>, Vec<u8>) {
        let full_skylight = &self.sky_light;
        let half_len = full_skylight.len() / 2;
        let mut top = Vec::with_capacity(half_len);
        let mut bottom = Vec::with_capacity(half_len);
        for i in 0..half_len {
            top.push(full_skylight[i * 2]);
            bottom.push(full_skylight[i * 2 + 1]);
        }
        (top, bottom)
    }
}

pub struct CachedLceChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    builder: LceChunkBuilder,
}

impl CachedLceChunk {
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self { chunk_x, chunk_z, builder: LceChunkBuilder::new() }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: u16) {
        self.builder.set_block(x, y, z, block);
    }

    pub fn snapshot_builder(&self) -> LceChunkBuilder {
        self.builder.snapshot()
    }
}

pub fn translate_java_chunk(
    _chunk_x: i32,
    _chunk_z: i32,
    chunk_data: &[u8],
) -> (LceChunkBuilder, Vec<u8>) {
    let mut builder = LceChunkBuilder::new();
    let mut offset: usize = 0;
    for section_y in 0..SECTIONS {
        if offset >= chunk_data.len() { break; }
        if offset + 2 > chunk_data.len() { break; }
        let block_count = i16::from_be_bytes([chunk_data[offset], chunk_data[offset + 1]]);
        offset += 2;
        if block_count == 0 {
            if offset >= chunk_data.len() { break; }
            let _bits_per_entry = chunk_data[offset];
            offset += 1;
            if offset >= chunk_data.len() { break; }
            let (palette_len, used) = read_varint_offset(chunk_data, offset);
            offset = used;
            offset += palette_len as usize * 4;
            if offset >= chunk_data.len() { break; }
            let (data_len, used2) = read_varint_offset(chunk_data, offset);
            offset = used2;
            offset += data_len as usize;
            continue;
        }
        if offset >= chunk_data.len() { break; }
        let bits_per_entry = chunk_data[offset];
        offset += 1;
        if bits_per_entry == 0 || bits_per_entry > 14 {
            if offset + 4 <= chunk_data.len() {
                offset += 4;
            }
            if offset >= chunk_data.len() { break; }
            let (data_len, used) = read_varint_offset(chunk_data, offset);
            offset = used;
            offset += data_len as usize;
            continue;
        }
        let (palette_len, used) = read_varint_offset(chunk_data, offset);
        offset = used;
        let mut palette = Vec::with_capacity(palette_len as usize);
        for _ in 0..palette_len {
            if offset + 4 > chunk_data.len() { break; }
            palette.push(i32::from_be_bytes([
                chunk_data[offset], chunk_data[offset+1], chunk_data[offset+2], chunk_data[offset+3]
            ]));
            offset += 4;
        }
        if offset >= chunk_data.len() { break; }
        let (data_len, used2) = read_varint_offset(chunk_data, offset);
        offset = used2;
        if offset + data_len as usize > chunk_data.len() { break; }
        let compact_data = &chunk_data[offset..offset + data_len as usize];
        offset += data_len as usize;
        let values_per_long = 64 / bits_per_entry as usize;
        let mask = (1u64 << bits_per_entry) - 1;
        for i in 0..4096usize {
            let long_idx = i / values_per_long;
            let bit_offset = (i % values_per_long) * bits_per_entry as usize;
            if long_idx >= compact_data.len() / 8 { continue; }
            let mut val = 0u64;
            for b in 0..8 {
                let byte_idx = long_idx * 8 + b;
                if byte_idx < compact_data.len() {
                    val |= (compact_data[byte_idx] as u64) << (b * 8);
                }
            }
            let palette_idx = ((val >> bit_offset) & mask) as usize;
            let java_state = palette.get(palette_idx).copied().unwrap_or(0);
            let bx = i & 0xF;
            let by = (i >> 8) & 0xF;
            let bz = (i >> 4) & 0xF;
            let world_y = section_y * 16 + by;
            let lce_block = registry::registry().get_lce(java_state);
            builder.set_block(bx, world_y, bz, lce_block);
        }
    }
    let raw_data = builder.build_raw_data();
    let compressed = compress_rle_zlib(&raw_data);
    (builder, compressed)
}

pub fn compress_rle_zlib(data: &[u8]) -> Vec<u8> {
    let rle_encoded = rle_encode(data);
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = encoder.write_all(&rle_encoded);
    encoder.finish().unwrap_or_else(|_| rle_encoded)
}

fn rle_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        if byte == 0xFF || (i + 1 < data.len() && data[i + 1] == byte) {
            let mut count = 1usize;
            while i + count < data.len() && data[i + count] == byte && count < 255 {
                count += 1;
            }
            out.push(0xFF);
            out.push((count - 1) as u8);
            out.push(byte);
            i += count;
        } else {
            out.push(byte);
            i += 1;
        }
    }
    out
}

fn read_varint_offset(data: &[u8], offset: usize) -> (i32, usize) {
    let mut result = 0i32;
    let mut shift = 0;
    let mut pos = offset;
    while pos < data.len() {
        let byte = data[pos];
        pos += 1;
        result |= ((byte & 0x7F) as i32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return (result, pos);
        }
        if shift >= 35 { break; }
    }
    (result, pos)
}
