use crate::arch::x86_64::io;
use crate::storage::block::{BlockDevice, BlockError};

const ATA_DATA: u16 = 0x1F0;
const ATA_SECTOR_COUNT: u16 = 0x1F2;
const ATA_LBA_LOW: u16 = 0x1F3;
const ATA_LBA_MID: u16 = 0x1F4;
const ATA_LBA_HIGH: u16 = 0x1F5;
const ATA_DRIVE_HEAD: u16 = 0x1F6;
const ATA_STATUS_COMMAND: u16 = 0x1F7;

const ATA_CMD_READ_SECTORS: u8 = 0x20;
const ATA_STATUS_ERR: u8 = 0x01;
const ATA_STATUS_DF: u8 = 0x20;
const ATA_STATUS_DRQ: u8 = 0x08;
const ATA_STATUS_BSY: u8 = 0x80;

const STATUS_POLL_LIMIT: usize = 100_000;

pub struct AtaPio {
    drive_select: u8,
}

impl AtaPio {
    pub fn primary_master() -> Self {
        Self { drive_select: 0xE0 }
    }

    fn wait_ready(&self) -> Result<u8, BlockError> {
        for _ in 0..STATUS_POLL_LIMIT {
            let status = io::inb(ATA_STATUS_COMMAND);
            if (status & ATA_STATUS_BSY) != 0 {
                continue;
            }
            if (status & ATA_STATUS_ERR) != 0 || (status & ATA_STATUS_DF) != 0 {
                return Err(BlockError::DeviceFault);
            }
            if (status & ATA_STATUS_DRQ) != 0 {
                return Ok(status);
            }
        }
        Err(BlockError::Timeout)
    }
}

impl BlockDevice for AtaPio {
    fn read_sector(&mut self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
        if out.len() != 512 {
            return Err(BlockError::InvalidBufferSize);
        }
        if lba > 0x0FFF_FFFF {
            return Err(BlockError::Unsupported);
        }

        let lba = lba as u32;
        io::outb(
            ATA_DRIVE_HEAD,
            self.drive_select | (((lba >> 24) as u8) & 0x0F),
        );
        io::outb(ATA_SECTOR_COUNT, 1);
        io::outb(ATA_LBA_LOW, (lba & 0xFF) as u8);
        io::outb(ATA_LBA_MID, ((lba >> 8) & 0xFF) as u8);
        io::outb(ATA_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);
        io::outb(ATA_STATUS_COMMAND, ATA_CMD_READ_SECTORS);

        self.wait_ready()?;

        for i in 0..256 {
            let word = io::inw(ATA_DATA);
            out[i * 2] = (word & 0x00FF) as u8;
            out[i * 2 + 1] = (word >> 8) as u8;
        }

        Ok(())
    }
}
