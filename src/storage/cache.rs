use alloc::vec::Vec;

use super::block::{BlockDevice, BlockError};

#[derive(Clone, Copy)]
struct CacheLine {
    valid: bool,
    lba: u64,
    last_use: u64,
    data: [u8; 512],
}

impl CacheLine {
    const fn empty() -> Self {
        Self {
            valid: false,
            lba: 0,
            last_use: 0,
            data: [0; 512],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

pub struct CachedBlockDevice<D: BlockDevice> {
    inner: D,
    lines: Vec<CacheLine>,
    ticks: u64,
    hits: u64,
    misses: u64,
}

impl<D: BlockDevice> CachedBlockDevice<D> {
    pub fn new(inner: D, capacity: usize) -> Self {
        let mut lines = Vec::with_capacity(capacity.max(1));
        for _ in 0..capacity.max(1) {
            lines.push(CacheLine::empty());
        }

        Self {
            inner,
            lines,
            ticks: 1,
            hits: 0,
            misses: 0,
        }
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
        }
    }
}

impl<D: BlockDevice> BlockDevice for CachedBlockDevice<D> {
    fn sector_size(&self) -> usize {
        self.inner.sector_size()
    }

    fn read_sector(&mut self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
        if out.len() != 512 {
            return Err(BlockError::InvalidBufferSize);
        }

        self.ticks = self.ticks.wrapping_add(1);

        if let Some(line) = self.lines.iter_mut().find(|l| l.valid && l.lba == lba) {
            self.hits += 1;
            line.last_use = self.ticks;
            out.copy_from_slice(&line.data);
            return Ok(());
        }

        self.misses += 1;

        let replace_idx = self
            .lines
            .iter()
            .enumerate()
            .min_by_key(|(_, line)| if line.valid { line.last_use } else { 0 })
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        self.inner.read_sector(lba, &mut self.lines[replace_idx].data)?;
        self.lines[replace_idx].valid = true;
        self.lines[replace_idx].lba = lba;
        self.lines[replace_idx].last_use = self.ticks;
        out.copy_from_slice(&self.lines[replace_idx].data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::CachedBlockDevice;
    use crate::storage::block::{BlockDevice, BlockError};

    struct MockDev {
        sectors: Vec<[u8; 512]>,
        reads: usize,
    }

    impl BlockDevice for MockDev {
        fn read_sector(&mut self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
            let src = self.sectors.get(lba as usize).ok_or(BlockError::Unsupported)?;
            out.copy_from_slice(src);
            self.reads += 1;
            Ok(())
        }
    }

    #[test]
    fn caches_repeated_sector_reads() {
        let mut s0 = [0_u8; 512];
        s0[0] = 42;
        let dev = MockDev {
            sectors: vec![s0],
            reads: 0,
        };
        let mut cached = CachedBlockDevice::new(dev, 4);
        let mut buf = [0_u8; 512];
        cached.read_sector(0, &mut buf).expect("read");
        cached.read_sector(0, &mut buf).expect("read");
        let stats = cached.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }
}
