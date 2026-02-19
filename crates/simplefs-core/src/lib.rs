#![cfg_attr(not(feature = "std"), no_std)]

pub const BLOCK_SIZE: usize = 512;
pub const MAGIC: [u8; 8] = *b"ERESFS1\0";
pub const VERSION: u32 = 1;
pub const DIR_ENTRY_NAME_LEN: usize = 32;
pub const DIR_ENTRY_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsError {
    InvalidMagic,
    InvalidVersion,
    InvalidBlockSize,
    InvalidData,
    NameTooLong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Superblock {
    pub magic: [u8; 8],
    pub version: u32,
    pub block_size: u32,
    pub total_blocks: u32,
    pub dir_entry_count: u32,
    pub dir_start_block: u32,
    pub dir_block_count: u32,
    pub data_start_block: u32,
}

impl Superblock {
    pub fn new(total_blocks: u32, dir_entry_count: u32, dir_block_count: u32) -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            block_size: BLOCK_SIZE as u32,
            total_blocks,
            dir_entry_count,
            dir_start_block: 1,
            dir_block_count,
            data_start_block: 1 + dir_block_count,
        }
    }

    pub fn encode(self, out: &mut [u8; BLOCK_SIZE]) {
        out.fill(0);
        out[0..8].copy_from_slice(&self.magic);
        write_u32(out, 8, self.version);
        write_u32(out, 12, self.block_size);
        write_u32(out, 16, self.total_blocks);
        write_u32(out, 20, self.dir_entry_count);
        write_u32(out, 24, self.dir_start_block);
        write_u32(out, 28, self.dir_block_count);
        write_u32(out, 32, self.data_start_block);
    }

    pub fn decode(input: &[u8; BLOCK_SIZE]) -> Result<Self, FsError> {
        let mut magic = [0_u8; 8];
        magic.copy_from_slice(&input[0..8]);
        let sb = Self {
            magic,
            version: read_u32(input, 8),
            block_size: read_u32(input, 12),
            total_blocks: read_u32(input, 16),
            dir_entry_count: read_u32(input, 20),
            dir_start_block: read_u32(input, 24),
            dir_block_count: read_u32(input, 28),
            data_start_block: read_u32(input, 32),
        };
        sb.validate()?;
        Ok(sb)
    }

    pub fn validate(&self) -> Result<(), FsError> {
        if self.magic != MAGIC {
            return Err(FsError::InvalidMagic);
        }
        if self.version != VERSION {
            return Err(FsError::InvalidVersion);
        }
        if self.block_size != BLOCK_SIZE as u32 {
            return Err(FsError::InvalidBlockSize);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: [u8; DIR_ENTRY_NAME_LEN],
    pub name_len: u8,
    pub file_start_block: u32,
    pub file_block_count: u32,
    pub file_size: u32,
    pub flags: u32,
}

impl DirEntry {
    pub fn new(name: &str, file_start_block: u32, file_block_count: u32, file_size: u32) -> Result<Self, FsError> {
        if name.is_empty() || name.len() > DIR_ENTRY_NAME_LEN {
            return Err(FsError::NameTooLong);
        }

        let mut out_name = [0_u8; DIR_ENTRY_NAME_LEN];
        out_name[..name.len()].copy_from_slice(name.as_bytes());
        Ok(Self {
            name: out_name,
            name_len: name.len() as u8,
            file_start_block,
            file_block_count,
            file_size,
            flags: 0,
        })
    }

    pub fn is_unused(&self) -> bool {
        self.name_len == 0
    }

    pub fn name(&self) -> Result<&str, FsError> {
        let len = self.name_len as usize;
        if len > DIR_ENTRY_NAME_LEN {
            return Err(FsError::InvalidData);
        }
        core::str::from_utf8(&self.name[..len]).map_err(|_| FsError::InvalidData)
    }

    pub fn encode(self, out: &mut [u8; DIR_ENTRY_SIZE]) {
        out.fill(0);
        out[0..DIR_ENTRY_NAME_LEN].copy_from_slice(&self.name);
        out[32] = self.name_len;
        write_u32(out, 36, self.file_start_block);
        write_u32(out, 40, self.file_block_count);
        write_u32(out, 44, self.file_size);
        write_u32(out, 48, self.flags);
    }

    pub fn decode(input: &[u8; DIR_ENTRY_SIZE]) -> Self {
        let mut name = [0_u8; DIR_ENTRY_NAME_LEN];
        name.copy_from_slice(&input[0..DIR_ENTRY_NAME_LEN]);
        Self {
            name,
            name_len: input[32],
            file_start_block: read_u32(input, 36),
            file_block_count: read_u32(input, 40),
            file_size: read_u32(input, 44),
            flags: read_u32(input, 48),
        }
    }
}

pub fn dir_blocks_for_entries(entry_count: usize) -> u32 {
    let bytes = entry_count.saturating_mul(DIR_ENTRY_SIZE);
    bytes.div_ceil(BLOCK_SIZE) as u32
}

pub fn blocks_for_size(size: usize) -> u32 {
    size.div_ceil(BLOCK_SIZE) as u32
}

fn write_u32(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::{blocks_for_size, dir_blocks_for_entries, DirEntry, Superblock, BLOCK_SIZE, DIR_ENTRY_SIZE};

    #[test]
    fn superblock_roundtrip() {
        let sb = Superblock::new(100, 3, 1);
        let mut buf = [0_u8; BLOCK_SIZE];
        sb.encode(&mut buf);
        let parsed = Superblock::decode(&buf).expect("decode");
        assert_eq!(parsed.total_blocks, 100);
        assert_eq!(parsed.dir_entry_count, 3);
    }

    #[test]
    fn dir_entry_roundtrip() {
        let entry = DirEntry::new("hello.txt", 3, 2, 700).expect("entry");
        let mut buf = [0_u8; DIR_ENTRY_SIZE];
        entry.encode(&mut buf);
        let parsed = DirEntry::decode(&buf);
        assert_eq!(parsed.name().expect("name"), "hello.txt");
        assert_eq!(parsed.file_size, 700);
    }

    #[test]
    fn computes_block_counts() {
        assert_eq!(dir_blocks_for_entries(8), 1);
        assert_eq!(dir_blocks_for_entries(9), 2);
        assert_eq!(blocks_for_size(0), 0);
        assert_eq!(blocks_for_size(513), 2);
    }
}
