use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;

use simplefs_core::{DirEntry, FsError, Superblock, BLOCK_SIZE, DIR_ENTRY_SIZE};

use crate::fs::vfs::{DirEntry as VfsDirEntry, FileSystem, Metadata, NodeId, NodeType, VfsError};
use crate::storage::block::{BlockDevice, BlockError};

pub struct SimpleFs<D: BlockDevice> {
    device: RefCell<D>,
    superblock: Superblock,
    entries: Vec<DirEntry>,
}

impl<D: BlockDevice> SimpleFs<D> {
    pub fn mount(mut device: D) -> Result<Self, VfsError> {
        // Block 0 contains the superblock with global FS layout metadata.
        let mut sector = [0_u8; BLOCK_SIZE];
        device.read_sector(0, &mut sector).map_err(map_block_error)?;
        let superblock = Superblock::decode(&sector).map_err(map_fs_error)?;

        // Directory data is stored as a contiguous block range right after the superblock.
        let dir_bytes = superblock.dir_block_count as usize * BLOCK_SIZE;
        let mut dir_data = vec![0_u8; dir_bytes];
        for i in 0..superblock.dir_block_count as usize {
            let start = i * BLOCK_SIZE;
            let end = start + BLOCK_SIZE;
            device
                .read_sector((superblock.dir_start_block as usize + i) as u64, &mut dir_data[start..end])
                .map_err(map_block_error)?;
        }

        let mut entries = Vec::new();
        for i in 0..superblock.dir_entry_count as usize {
            let start = i * DIR_ENTRY_SIZE;
            let end = start + DIR_ENTRY_SIZE;
            if end > dir_data.len() {
                return Err(VfsError::Io);
            }
            let mut raw = [0_u8; DIR_ENTRY_SIZE];
            raw.copy_from_slice(&dir_data[start..end]);
            let entry = DirEntry::decode(&raw);
            if !entry.is_unused() {
                entries.push(entry);
            }
        }

        Ok(Self {
            device: RefCell::new(device),
            superblock,
            entries,
        })
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn superblock(&self) -> Superblock {
        self.superblock
    }

    fn entry_node(index: usize) -> NodeId {
        NodeId((index + 1) as u64)
    }

    fn node_entry_index(node: NodeId) -> Option<usize> {
        if node.0 == 0 {
            None
        } else {
            Some((node.0 - 1) as usize)
        }
    }

    fn entry_name(entry: &DirEntry) -> Option<&str> {
        entry.name().ok()
    }
}

fn map_block_error(err: BlockError) -> VfsError {
    match err {
        BlockError::InvalidBufferSize | BlockError::DeviceFault | BlockError::Timeout => VfsError::Io,
        BlockError::Unsupported => VfsError::Unsupported,
    }
}

fn map_fs_error(err: FsError) -> VfsError {
    match err {
        FsError::InvalidMagic | FsError::InvalidVersion | FsError::InvalidBlockSize | FsError::InvalidData => VfsError::Io,
        FsError::NameTooLong => VfsError::InvalidPath,
    }
}

impl<D: BlockDevice> FileSystem for SimpleFs<D> {
    fn root(&self) -> NodeId {
        NodeId(0)
    }

    fn lookup(&self, parent: NodeId, name: &str) -> Result<NodeId, VfsError> {
        if parent.0 != 0 {
            return Err(VfsError::NotDirectory);
        }

        for (i, entry) in self.entries.iter().enumerate() {
            if Self::entry_name(entry) == Some(name) {
                return Ok(Self::entry_node(i));
            }
        }
        Err(VfsError::NotFound)
    }

    fn metadata(&self, node: NodeId) -> Result<Metadata, VfsError> {
        if node.0 == 0 {
            return Ok(Metadata {
                node_type: NodeType::Directory,
                size: self.entries.len() as u64,
            });
        }

        let index = Self::node_entry_index(node).ok_or(VfsError::NotFound)?;
        let entry = self.entries.get(index).ok_or(VfsError::NotFound)?;
        Ok(Metadata {
            node_type: NodeType::File,
            size: entry.file_size as u64,
        })
    }

    fn read(&self, node: NodeId, offset: u64, out: &mut [u8]) -> Result<usize, VfsError> {
        if node.0 == 0 {
            return Err(VfsError::NotFile);
        }

        let index = Self::node_entry_index(node).ok_or(VfsError::NotFound)?;
        let entry = self.entries.get(index).ok_or(VfsError::NotFound)?;
        if offset >= entry.file_size as u64 {
            return Ok(0);
        }

        let mut read_total = 0_usize;
        let max_bytes = core::cmp::min(out.len(), entry.file_size as usize - offset as usize);
        let mut cursor = offset as usize;
        let mut scratch = [0_u8; BLOCK_SIZE];
        while read_total < max_bytes {
            // Translate file offset -> (disk block, in-block offset).
            let abs = cursor;
            let block_index = abs / BLOCK_SIZE;
            let block_offset = abs % BLOCK_SIZE;
            let lba = entry.file_start_block as usize + block_index;
            self.device
                .borrow_mut()
                .read_sector(lba as u64, &mut scratch)
                .map_err(map_block_error)?;

            let to_copy = core::cmp::min(max_bytes - read_total, BLOCK_SIZE - block_offset);
            out[read_total..read_total + to_copy]
                .copy_from_slice(&scratch[block_offset..block_offset + to_copy]);
            read_total += to_copy;
            cursor += to_copy;
        }

        Ok(read_total)
    }

    fn list(&self, dir: NodeId) -> Result<Vec<VfsDirEntry>, VfsError> {
        if dir.0 != 0 {
            return Err(VfsError::NotDirectory);
        }

        let mut out = Vec::new();
        for (i, entry) in self.entries.iter().enumerate() {
            let name = Self::entry_name(entry).ok_or(VfsError::Io)?;
            out.push(VfsDirEntry::new(name, Self::entry_node(i), NodeType::File)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use simplefs_core::{
        blocks_for_size, dir_blocks_for_entries, DirEntry, Superblock, BLOCK_SIZE, DIR_ENTRY_SIZE,
    };
    use simplefs_tool::build_image_from_paths;

    use crate::fs::vfs::FileSystem;
    use crate::storage::block::{BlockDevice, BlockError};

    use super::SimpleFs;

    struct MemDisk {
        sectors: Vec<[u8; BLOCK_SIZE]>,
    }

    impl BlockDevice for MemDisk {
        fn read_sector(&mut self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
            if out.len() != BLOCK_SIZE {
                return Err(BlockError::InvalidBufferSize);
            }
            let s = self.sectors.get(lba as usize).ok_or(BlockError::Unsupported)?;
            out.copy_from_slice(s);
            Ok(())
        }
    }

    #[test]
    fn mounts_simple_image() {
        let data = b"hello";
        let dir_blocks = dir_blocks_for_entries(1);
        let total_blocks = 1 + dir_blocks + blocks_for_size(data.len());
        let sb = Superblock::new(total_blocks, 1, dir_blocks);
        let entry = DirEntry::new("greet.txt", sb.data_start_block, blocks_for_size(data.len()), data.len() as u32)
            .expect("entry");

        let mut sectors = vec![[0_u8; BLOCK_SIZE]; total_blocks as usize];
        sb.encode(&mut sectors[0]);
        let mut raw_entry = [0_u8; DIR_ENTRY_SIZE];
        entry.encode(&mut raw_entry);
        sectors[1][..DIR_ENTRY_SIZE].copy_from_slice(&raw_entry);
        sectors[sb.data_start_block as usize][..data.len()].copy_from_slice(data);

        let fs = SimpleFs::mount(MemDisk { sectors }).expect("mount");
        assert_eq!(fs.entry_count(), 1);
    }

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        path.push(format!("eres-os-simplefs-test-{name}-{nanos}"));
        path
    }

    fn sectors_from_image(image: &[u8]) -> Vec<[u8; BLOCK_SIZE]> {
        assert_eq!(image.len() % BLOCK_SIZE, 0);
        let mut sectors = Vec::new();
        for chunk in image.chunks_exact(BLOCK_SIZE) {
            let mut sector = [0_u8; BLOCK_SIZE];
            sector.copy_from_slice(chunk);
            sectors.push(sector);
        }
        sectors
    }

    #[test]
    fn tool_generated_image_mounts_and_reads_file() {
        let dir = temp_path("input");
        fs::create_dir_all(&dir).expect("create dir");
        let hello = dir.join("hello.txt");
        let notes = dir.join("notes.txt");
        fs::write(&hello, b"hello from tool").expect("write hello");
        fs::write(&notes, b"notes").expect("write notes");

        let sources = vec![hello, notes];
        let image = build_image_from_paths(&sources).expect("build image");
        let sectors = sectors_from_image(&image);
        let fs = SimpleFs::mount(MemDisk { sectors }).expect("mount");

        let node = fs.lookup(fs.root(), "hello.txt").expect("lookup file");
        let meta = fs.metadata(node).expect("metadata");
        assert_eq!(meta.size, 15);

        let mut out = [0_u8; 32];
        let read = fs.read(node, 0, &mut out).expect("read");
        assert_eq!(&out[..read], b"hello from tool");

        let _ = fs::remove_dir_all(dir);
    }
}
