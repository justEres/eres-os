use crate::storage::block::BlockError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    Block(BlockError),
    Corrupt,
    Unsupported,
}

impl From<BlockError> for StorageError {
    fn from(value: BlockError) -> Self {
        Self::Block(value)
    }
}
