#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockError {
    InvalidBufferSize,
    DeviceFault,
    Timeout,
    Unsupported,
}

pub trait BlockDevice {
    fn sector_size(&self) -> usize {
        512
    }

    fn read_sector(&mut self, lba: u64, out: &mut [u8]) -> Result<(), BlockError>;
}

#[cfg(test)]
mod tests {
    use super::{BlockDevice, BlockError};

    struct MockBlock {
        sector0: [u8; 512],
    }

    impl MockBlock {
        fn new() -> Self {
            let mut sector0 = [0_u8; 512];
            sector0[510] = 0x55;
            sector0[511] = 0xAA;
            Self { sector0 }
        }
    }

    impl BlockDevice for MockBlock {
        fn read_sector(&mut self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
            if out.len() != 512 {
                return Err(BlockError::InvalidBufferSize);
            }
            if lba != 0 {
                return Err(BlockError::Unsupported);
            }
            out.copy_from_slice(&self.sector0);
            Ok(())
        }
    }

    #[test]
    fn reads_boot_signature_from_mock_device() {
        let mut dev = MockBlock::new();
        let mut buf = [0_u8; 512];
        dev.read_sector(0, &mut buf).expect("sector read");
        assert_eq!((buf[510], buf[511]), (0x55, 0xAA));
    }

    #[test]
    fn rejects_wrong_buffer_size() {
        let mut dev = MockBlock::new();
        let mut buf = [0_u8; 128];
        assert_eq!(
            dev.read_sector(0, &mut buf),
            Err(BlockError::InvalidBufferSize)
        );
    }
}
