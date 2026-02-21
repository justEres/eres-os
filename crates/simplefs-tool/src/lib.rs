use clap::Parser;
use simplefs_core::{blocks_for_size, dir_blocks_for_entries, DirEntry, Superblock, BLOCK_SIZE, DIR_ENTRY_SIZE};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Parser)]
#[command(name = "simplefs-tool", about = "Build a simplefs disk image from host files")]
pub struct Cli {
    /// Output disk image path.
    #[arg(short, long, value_name = "IMG")]
    pub output: PathBuf,
    /// Explicit input file (repeatable).
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub files: Vec<PathBuf>,
    /// Include all regular files from this directory.
    #[arg(long = "input-dir", value_name = "DIR")]
    pub input_dir: Option<PathBuf>,
}

#[derive(Debug)]
struct InputFile {
    name: String,
    data: Vec<u8>,
}

pub fn run_from<I, T>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    run_with_cli(cli)
}

pub fn run_with_cli(cli: Cli) -> Result<(), String> {
    let sources = collect_sources(&cli.files, cli.input_dir.as_deref())?;
    if sources.is_empty() {
        return Err("at least one input file is required (use --file or --input-dir)".to_string());
    }

    write_image(&cli.output, &sources)?;
    println!("wrote {}", cli.output.display());
    Ok(())
}

pub fn write_image(output: &Path, sources: &[PathBuf]) -> Result<(), String> {
    let image = build_image_from_paths(sources)?;
    fs::write(output, image).map_err(|e| format!("write {}: {e}", output.display()))
}

pub fn build_image_from_paths(sources: &[PathBuf]) -> Result<Vec<u8>, String> {
    let mut files = Vec::new();
    for source in sources {
        files.push(load_input_file(source)?);
    }
    build_image(&files)
}

pub fn collect_sources(files: &[PathBuf], input_dir: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    if let Some(dir) = input_dir {
        let read_dir = fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
        for entry in read_dir {
            let entry = entry.map_err(|e| format!("read dir entry {}: {e}", dir.display()))?;
            let ty = entry
                .file_type()
                .map_err(|e| format!("read file type {}: {e}", entry.path().display()))?;
            if ty.is_file() {
                out.push(entry.path());
            }
        }
    }

    out.extend(files.iter().cloned());
    out.sort();
    // Avoid writing duplicate directory entries when both --file and --input-dir
    // include the same path.
    out.dedup();
    Ok(out)
}

fn load_input_file(path: &Path) -> Result<InputFile, String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("invalid filename: {}", path.display()))?
        .to_string_lossy()
        .to_string();
    let data = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(InputFile { name, data })
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

#[cfg(test)]
mod tests {
    use super::{collect_sources, run_from};
    use simplefs_core::Superblock;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        path.push(format!("eres-os-simplefs-{name}-{nanos}"));
        path
    }

    #[test]
    fn collects_from_directory_and_files() {
        let dir = temp_path("collect");
        fs::create_dir_all(&dir).expect("create dir");
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, b"a").expect("write a");
        fs::write(&b, b"b").expect("write b");

        let extra = temp_path("extra.txt");
        fs::write(&extra, b"x").expect("write extra");

        let sources = collect_sources(std::slice::from_ref(&extra), Some(&dir)).expect("collect");
        assert!(sources.iter().any(|p| p.ends_with("a.txt")));
        assert!(sources.iter().any(|p| p.ends_with("b.txt")));
        assert!(sources.iter().any(|p| p == &extra));

        let _ = fs::remove_file(extra);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn builds_image_via_cli_flags() {
        let dir = temp_path("cli-input");
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join("hello.txt"), b"hello").expect("write hello");
        fs::write(dir.join("world.txt"), b"world").expect("write world");

        let out = temp_path("cli.img");
        let args = [
            "simplefs-tool",
            "--output",
            out.to_str().expect("out str"),
            "--input-dir",
            dir.to_str().expect("dir str"),
        ];
        run_from(args).expect("run cli");

        let image = fs::read(&out).expect("read image");
        let mut sb_buf = [0_u8; simplefs_core::BLOCK_SIZE];
        sb_buf.copy_from_slice(&image[..simplefs_core::BLOCK_SIZE]);
        let sb = Superblock::decode(&sb_buf).expect("decode superblock");
        assert_eq!(sb.dir_entry_count, 2);

        let _ = fs::remove_file(out);
        let _ = fs::remove_dir_all(dir);
    }
}
