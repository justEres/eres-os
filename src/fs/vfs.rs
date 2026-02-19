use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    AlreadyExists,
    InvalidPath,
    NotDirectory,
    NotFile,
    Io,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub node_type: NodeType,
    pub size: u64,
}

pub trait FileSystem {
    fn root(&self) -> NodeId;
    fn lookup(&self, parent: NodeId, name: &str) -> Result<NodeId, VfsError>;
    fn metadata(&self, node: NodeId) -> Result<Metadata, VfsError>;
    fn read(&self, node: NodeId, offset: u64, out: &mut [u8]) -> Result<usize, VfsError>;
    fn list(&self, dir: NodeId) -> Result<Vec<DirEntry>, VfsError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: [u8; 32],
    pub name_len: usize,
    pub node: NodeId,
    pub node_type: NodeType,
}

impl DirEntry {
    pub fn new(name: &str, node: NodeId, node_type: NodeType) -> Result<Self, VfsError> {
        if name.is_empty() || name.len() > 32 {
            return Err(VfsError::InvalidPath);
        }

        let mut out = [0_u8; 32];
        out[..name.len()].copy_from_slice(name.as_bytes());
        Ok(Self {
            name: out,
            name_len: name.len(),
            node,
            node_type,
        })
    }

    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("?")
    }
}

pub fn split_path(path: &str) -> Result<Vec<&str>, VfsError> {
    if !path.starts_with('/') {
        return Err(VfsError::InvalidPath);
    }

    let mut out = Vec::new();
    for part in path.split('/') {
        if part.is_empty() {
            continue;
        }
        if part == "." || part == ".." {
            return Err(VfsError::InvalidPath);
        }
        out.push(part);
    }
    Ok(out)
}

pub fn resolve_path<F: FileSystem + ?Sized>(fs: &F, path: &str) -> Result<NodeId, VfsError> {
    let parts = split_path(path)?;
    let mut current = fs.root();

    for part in parts {
        let meta = fs.metadata(current)?;
        if meta.node_type != NodeType::Directory {
            return Err(VfsError::NotDirectory);
        }
        current = fs.lookup(current, part)?;
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::{resolve_path, split_path, DirEntry, FileSystem, Metadata, NodeId, NodeType, VfsError};

    struct MockFs;

    impl FileSystem for MockFs {
        fn root(&self) -> NodeId {
            NodeId(1)
        }

        fn lookup(&self, parent: NodeId, name: &str) -> Result<NodeId, VfsError> {
            match (parent.0, name) {
                (1, "etc") => Ok(NodeId(2)),
                (2, "hosts") => Ok(NodeId(3)),
                _ => Err(VfsError::NotFound),
            }
        }

        fn metadata(&self, node: NodeId) -> Result<Metadata, VfsError> {
            match node.0 {
                1 | 2 => Ok(Metadata {
                    node_type: NodeType::Directory,
                    size: 0,
                }),
                3 => Ok(Metadata {
                    node_type: NodeType::File,
                    size: 10,
                }),
                _ => Err(VfsError::NotFound),
            }
        }

        fn read(&self, _node: NodeId, _offset: u64, _out: &mut [u8]) -> Result<usize, VfsError> {
            Ok(0)
        }

        fn list(&self, _dir: NodeId) -> Result<Vec<DirEntry>, VfsError> {
            Ok(vec![])
        }
    }

    #[test]
    fn splits_absolute_path() {
        let parts = split_path("/etc/hosts").expect("split");
        assert_eq!(parts, vec!["etc", "hosts"]);
    }

    #[test]
    fn rejects_relative_path() {
        assert_eq!(split_path("etc/hosts"), Err(VfsError::InvalidPath));
    }

    #[test]
    fn resolves_existing_path() {
        let fs = MockFs;
        assert_eq!(resolve_path(&fs, "/etc/hosts"), Ok(NodeId(3)));
    }

    #[test]
    fn creates_dir_entry() {
        let entry = DirEntry::new("init", NodeId(7), NodeType::File).expect("entry");
        assert_eq!(entry.name(), "init");
        assert_eq!(entry.node, NodeId(7));
    }
}
