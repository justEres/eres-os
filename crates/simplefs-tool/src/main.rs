use simplefs_core::{blocks_for_size, dir_blocks_for_entries, DirEntry, Superblock, BLOCK_SIZE, DIR_ENTRY_SIZE};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct InputFile {
    source: PathBuf,
    name: String,
    data: Vec<u8>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("simplefs-tool: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(output) = args.next() else {
        return Err("usage: simplefs-tool <output.img> <file> [file...]".to_string());
    };

    let sources: Vec<PathBuf> = args.map(PathBuf::from).collect();
    if sources.is_empty() {
        return Err("at least one input file is required".to_string());
    }

    let mut files = Vec::new();
    for source in sources {
        files.push(load_input_file(&source)?);
    }

    let image = build_image(&files)?;
    fs::write(&output, image).map_err(|e| format!("write {}: {e}", output))?;
    println!("wrote {}", output);
    Ok(())
}

fn load_input_file(path: &Path) -> Result<InputFile, String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("invalid filename: {}", path.display()))?
        .to_string_lossy()
        .to_string();
    let data = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(InputFile {
        source: path.to_path_buf(),
        name,
        data,
    })
}

fn build_image(files: &[InputFile]) -> Result<Vec<u8>, String> {
    let dir_blocks = dir_blocks_for_entries(files.len()) as usize;
    let mut current_data_block = 1 + dir_blocks as u32;
    let mut entries = Vec::new();
    let mut total_data_blocks = 0_u32;

    for file in files {
        let blocks = blocks_for_size(file.data.len());
        entries.push(
            DirEntry::new(
                &file.name,
                current_data_block,
                blocks,
                file.data.len() as u32,
            )
            .map_err(|_| format!("invalid entry name: {}", file.name))?,
        );
        current_data_block += blocks;
        total_data_blocks += blocks;
    }

    let total_blocks = 1 + dir_blocks as u32 + total_data_blocks;
    let sb = Superblock::new(total_blocks, entries.len() as u32, dir_blocks as u32);
    let mut image = vec![0_u8; total_blocks as usize * BLOCK_SIZE];

    let mut sb_sector = [0_u8; BLOCK_SIZE];
    sb.encode(&mut sb_sector);
    image[0..BLOCK_SIZE].copy_from_slice(&sb_sector);

    for (i, entry) in entries.iter().enumerate() {
        let mut encoded = [0_u8; DIR_ENTRY_SIZE];
        entry.encode(&mut encoded);
        let offset = BLOCK_SIZE + i * DIR_ENTRY_SIZE;
        image[offset..offset + DIR_ENTRY_SIZE].copy_from_slice(&encoded);
    }

    for (file, entry) in files.iter().zip(entries.iter()) {
        let start = entry.file_start_block as usize * BLOCK_SIZE;
        let end = start + file.data.len();
        image[start..end].copy_from_slice(&file.data);
    }

    Ok(image)
}
