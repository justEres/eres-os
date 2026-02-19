use crate::{arch, console};
use alloc::string::String;
use alloc::vec::Vec;
use core::arch::asm;

#[cfg(eres_kernel)]
use crate::fs::simplefs::SimpleFs;
#[cfg(eres_kernel)]
use crate::fs::vfs::{resolve_path, FileSystem, NodeType};
#[cfg(eres_kernel)]
use crate::storage::ata_pio::AtaPio;
#[cfg(eres_kernel)]
use crate::storage::cache::CachedBlockDevice;

const MAX_LINE: usize = 128;
const MAX_HISTORY: usize = 16;
const HELP_TEXT: &[u8] = b"commands: help echo clear history mem ticks ls cat stat panic halt reboot";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    Empty,
    Help,
    Echo,
    Clear,
    History,
    Mem,
    Ticks,
    Ls,
    Cat,
    Stat,
    Panic,
    Halt,
    Reboot,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParsedCommand<'a> {
    kind: CommandKind,
    arg: &'a [u8],
}

pub fn run() -> ! {
    let mut line_buf = [0_u8; MAX_LINE];
    let mut len = 0_usize;
    let mut history: Vec<Vec<u8>> = Vec::new();
    let mut history_index: Option<usize> = None;

    console::write_line(b"Type 'help' for commands.");
    prompt();

    loop {
        if let Some(key) = arch::x86_64::keyboard::try_read_key() {
            match key {
                arch::x86_64::keyboard::KeyEvent::Enter => {
                    console::write_byte(b'\n');
                    execute_command(&line_buf[..len], &mut history);
                    len = 0;
                    history_index = None;
                    prompt();
                }
                arch::x86_64::keyboard::KeyEvent::Backspace => {
                    if len > 0 {
                        len -= 1;
                        console::backspace();
                    }
                    history_index = None;
                }
                arch::x86_64::keyboard::KeyEvent::Char(b) if is_printable_ascii(b) => {
                    if len < MAX_LINE {
                        line_buf[len] = b;
                        len += 1;
                        console::write_byte(b);
                    }
                    history_index = None;
                }
                arch::x86_64::keyboard::KeyEvent::Up => {
                    if history.is_empty() {
                        continue;
                    }

                    history_index = Some(match history_index {
                        Some(current) if current > 0 => current - 1,
                        Some(current) => current,
                        None => history.len() - 1,
                    });

                    if let Some(index) = history_index {
                        replace_line(&mut line_buf, &mut len, &history[index]);
                    }
                }
                arch::x86_64::keyboard::KeyEvent::Down => {
                    if history.is_empty() {
                        continue;
                    }

                    match history_index {
                        Some(current) if current + 1 < history.len() => {
                            history_index = Some(current + 1);
                            replace_line(&mut line_buf, &mut len, &history[current + 1]);
                        }
                        Some(_) => {
                            history_index = None;
                            replace_line(&mut line_buf, &mut len, b"");
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        } else {
            arch::x86_64::halt();
        }
    }
}

fn prompt() {
    console::write_str(b"> ");
}

fn execute_command(line: &[u8], history: &mut Vec<Vec<u8>>) {
    let parsed = parse_command(line);

    if !line.is_empty() {
        if history.len() >= MAX_HISTORY {
            let _ = history.remove(0);
        }
        history.push(line.to_vec());
    }

    match parsed.kind {
        CommandKind::Empty => {}
        CommandKind::Help => {
            console::write_line(HELP_TEXT);
        }
        CommandKind::Echo => {
            console::write_line(parsed.arg);
        }
        CommandKind::Clear => {
            console::clear();
        }
        CommandKind::History => {
            for entry in history.iter() {
                console::write_line(entry);
            }
        }
        CommandKind::Ticks => {
            console::write_str(b"ticks=");
            console::write_u64(arch::x86_64::pit::ticks());
            console::write_byte(b'\n');
        }
        CommandKind::Mem => {
            if let Some(stats) = crate::memory::frame_allocator::stats() {
                console::write_str(b"frames total=");
                console::write_u64(stats.total_frames);
                console::write_str(b" allocated=");
                console::write_u64(stats.allocated_frames);
                console::write_str(b" free=");
                console::write_u64(stats.free_frames);
                console::write_byte(b'\n');
            } else {
                console::write_line(b"frame allocator not initialized");
            }
        }
        CommandKind::Ls => {
            let path = match core::str::from_utf8(parsed.arg) {
                Ok(path) => path,
                Err(_) => {
                    console::write_line(b"invalid path");
                    return;
                }
            };
            run_ls(path);
        }
        CommandKind::Cat => {
            let path = match core::str::from_utf8(parsed.arg) {
                Ok(path) => path,
                Err(_) => {
                    console::write_line(b"invalid path");
                    return;
                }
            };
            run_cat(path);
        }
        CommandKind::Stat => {
            let path = match core::str::from_utf8(parsed.arg) {
                Ok(path) => path,
                Err(_) => {
                    console::write_line(b"invalid path");
                    return;
                }
            };
            run_stat(path);
        }
        CommandKind::Panic => {
            unsafe {
                asm!("ud2", options(nomem, nostack, preserves_flags));
            }
            arch::x86_64::hang();
        }
        CommandKind::Halt => {
            console::write_line(b"Halting CPU.");
            arch::x86_64::hang();
        }
        CommandKind::Reboot => {
            console::write_line(b"Rebooting.");
            arch::x86_64::reboot();
        }
        CommandKind::Unknown => {
            console::write_line(b"unknown command");
        }
    }
}

#[cfg(eres_kernel)]
type FsDevice = CachedBlockDevice<AtaPio>;

#[cfg(eres_kernel)]
fn mount_simplefs() -> Result<SimpleFs<FsDevice>, &'static [u8]> {
    let dev = CachedBlockDevice::new(AtaPio::primary_slave(), 16);
    SimpleFs::mount(dev).map_err(|_| b"simplefs unavailable".as_slice())
}

#[cfg(eres_kernel)]
fn resolve_simplefs_path(fs: &SimpleFs<FsDevice>, path: &str) -> Result<crate::fs::vfs::NodeId, &'static [u8]> {
    let normalized = normalize_simplefs_path(path);
    if normalized == "/" {
        Ok(fs.root())
    } else {
        resolve_path(fs, &normalized).map_err(|_| b"path not found".as_slice())
    }
}

fn normalize_simplefs_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return String::from("/");
    }

    if trimmed.as_bytes()[0] == b'/' {
        String::from(trimmed)
    } else {
        let mut out = String::from("/");
        out.push_str(trimmed);
        out
    }
}

#[cfg(eres_kernel)]
fn run_ls(path: &str) {
    let Ok(fs) = mount_simplefs() else {
        console::write_line(b"simplefs unavailable");
        return;
    };

    let Ok(node) = resolve_simplefs_path(&fs, path) else {
        console::write_line(b"path not found");
        return;
    };

    let Ok(meta) = fs.metadata(node) else {
        console::write_line(b"stat failed");
        return;
    };

    if meta.node_type != NodeType::Directory {
        console::write_line(b"not a directory");
        return;
    }

    let Ok(entries) = fs.list(node) else {
        console::write_line(b"list failed");
        return;
    };

    if entries.is_empty() {
        console::write_line(b"(empty)");
        return;
    }

    for entry in entries {
        console::write_line(entry.name().as_bytes());
    }
}

#[cfg(not(eres_kernel))]
fn run_ls(_path: &str) {
    console::write_line(b"simplefs unavailable");
}

#[cfg(eres_kernel)]
fn run_cat(path: &str) {
    let Ok(fs) = mount_simplefs() else {
        console::write_line(b"simplefs unavailable");
        return;
    };

    let Ok(node) = resolve_simplefs_path(&fs, path) else {
        console::write_line(b"path not found");
        return;
    };

    let Ok(meta) = fs.metadata(node) else {
        console::write_line(b"stat failed");
        return;
    };

    if meta.node_type != NodeType::File {
        console::write_line(b"not a file");
        return;
    }

    let size = meta.size as usize;
    let mut buffer = Vec::new();
    buffer.resize(size, 0);

    let mut total = 0_usize;
    while total < size {
        match fs.read(node, total as u64, &mut buffer[total..]) {
            Ok(0) => break,
            Ok(read) => total += read,
            Err(_) => {
                console::write_line(b"read failed");
                return;
            }
        }
    }

    if total > 0 {
        console::write_str(&buffer[..total]);
    }
    if total == 0 || buffer[total - 1] != b'\n' {
        console::write_byte(b'\n');
    }
}

#[cfg(not(eres_kernel))]
fn run_cat(_path: &str) {
    console::write_line(b"simplefs unavailable");
}

#[cfg(eres_kernel)]
fn run_stat(path: &str) {
    let Ok(fs) = mount_simplefs() else {
        console::write_line(b"simplefs unavailable");
        return;
    };

    let Ok(node) = resolve_simplefs_path(&fs, path) else {
        console::write_line(b"path not found");
        return;
    };

    let Ok(meta) = fs.metadata(node) else {
        console::write_line(b"stat failed");
        return;
    };

    let kind = if meta.node_type == NodeType::Directory {
        b"directory".as_slice()
    } else {
        b"file".as_slice()
    };

    console::write_str(b"type=");
    console::write_str(kind);
    console::write_str(b" size=");
    console::write_u64(meta.size);
    console::write_byte(b'\n');
}

#[cfg(not(eres_kernel))]
fn run_stat(_path: &str) {
    console::write_line(b"simplefs unavailable");
}

fn parse_command(line: &[u8]) -> ParsedCommand<'_> {
    let trimmed = trim_spaces(line);
    if trimmed.is_empty() {
        return ParsedCommand {
            kind: CommandKind::Empty,
            arg: b"",
        };
    }

    let (cmd, arg) = split_cmd_arg(trimmed);
    match cmd {
        b"help" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::Help,
            arg: b"",
        },
        b"clear" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::Clear,
            arg: b"",
        },
        b"history" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::History,
            arg: b"",
        },
        b"mem" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::Mem,
            arg: b"",
        },
        b"ticks" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::Ticks,
            arg: b"",
        },
        b"panic" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::Panic,
            arg: b"",
        },
        b"halt" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::Halt,
            arg: b"",
        },
        b"reboot" if arg.is_empty() => ParsedCommand {
            kind: CommandKind::Reboot,
            arg: b"",
        },
        b"echo" if !arg.is_empty() => ParsedCommand {
            kind: CommandKind::Echo,
            arg,
        },
        b"ls" => ParsedCommand {
            kind: CommandKind::Ls,
            arg: if arg.is_empty() { b"/" } else { arg },
        },
        b"cat" if !arg.is_empty() => ParsedCommand {
            kind: CommandKind::Cat,
            arg,
        },
        b"stat" if !arg.is_empty() => ParsedCommand {
            kind: CommandKind::Stat,
            arg,
        },
        _ => ParsedCommand {
            kind: CommandKind::Unknown,
            arg: b"",
        },
    }
}

#[cfg(any(test, feature = "qemu-test"))]
pub fn run_command_self_tests() -> bool {
    let mut ok = true;
    ok &= check_parse(b"", CommandKind::Empty, b"");
    ok &= check_parse(b"help", CommandKind::Help, b"");
    ok &= check_parse(b"clear", CommandKind::Clear, b"");
    ok &= check_parse(b"history", CommandKind::History, b"");
    ok &= check_parse(b"mem", CommandKind::Mem, b"");
    ok &= check_parse(b"ticks", CommandKind::Ticks, b"");
    ok &= check_parse(b"panic", CommandKind::Panic, b"");
    ok &= check_parse(b"halt", CommandKind::Halt, b"");
    ok &= check_parse(b"reboot", CommandKind::Reboot, b"");
    ok &= check_parse(b"echo hello", CommandKind::Echo, b"hello");
    ok &= check_parse(b"ls", CommandKind::Ls, b"/");
    ok &= check_parse(b"ls /", CommandKind::Ls, b"/");
    ok &= check_parse(b"cat /motd.txt", CommandKind::Cat, b"/motd.txt");
    ok &= check_parse(b"stat /motd.txt", CommandKind::Stat, b"/motd.txt");
    ok &= check_parse(b"echo", CommandKind::Unknown, b"");
    ok &= check_parse(b"cat", CommandKind::Unknown, b"");
    ok &= check_parse(b"stat", CommandKind::Unknown, b"");
    ok &= check_parse(b"unknown", CommandKind::Unknown, b"");
    ok
}

#[cfg(any(test, feature = "qemu-test"))]
fn check_parse(line: &[u8], expected_kind: CommandKind, expected_arg: &[u8]) -> bool {
    let parsed = parse_command(line);
    parsed.kind == expected_kind && parsed.arg == expected_arg
}

fn is_printable_ascii(byte: u8) -> bool {
    (0x20..=0x7E).contains(&byte)
}

fn trim_spaces(mut input: &[u8]) -> &[u8] {
    while let Some((first, rest)) = input.split_first() {
        if *first == b' ' {
            input = rest;
        } else {
            break;
        }
    }

    while matches!(input.last(), Some(b' ')) {
        input = &input[..input.len() - 1];
    }

    input
}

fn split_cmd_arg(input: &[u8]) -> (&[u8], &[u8]) {
    match input.iter().position(|b| *b == b' ') {
        Some(space) => {
            let cmd = &input[..space];
            let arg = trim_spaces(&input[space + 1..]);
            (cmd, arg)
        }
        None => (input, b""),
    }
}

fn replace_line(line_buf: &mut [u8], len: &mut usize, replacement: &[u8]) {
    while *len > 0 {
        console::backspace();
        *len -= 1;
    }

    let copy_len = replacement.len().min(line_buf.len());
    line_buf[..copy_len].copy_from_slice(&replacement[..copy_len]);
    *len = copy_len;
    for byte in &replacement[..copy_len] {
        console::write_byte(*byte);
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_simplefs_path, parse_command, CommandKind};

    #[test]
    fn parses_help() {
        let parsed = parse_command(b"help");
        assert_eq!(parsed.kind, CommandKind::Help);
        assert_eq!(parsed.arg, b"");
    }

    #[test]
    fn parses_echo_argument() {
        let parsed = parse_command(b"echo hallo");
        assert_eq!(parsed.kind, CommandKind::Echo);
        assert_eq!(parsed.arg, b"hallo");
    }

    #[test]
    fn parses_unknown_command() {
        let parsed = parse_command(b"foo");
        assert_eq!(parsed.kind, CommandKind::Unknown);
        assert_eq!(parsed.arg, b"");
    }

    #[test]
    fn parses_history_command() {
        let parsed = parse_command(b"history");
        assert_eq!(parsed.kind, CommandKind::History);
        assert_eq!(parsed.arg, b"");
    }

    #[test]
    fn parses_ticks_command() {
        let parsed = parse_command(b"ticks");
        assert_eq!(parsed.kind, CommandKind::Ticks);
        assert_eq!(parsed.arg, b"");
    }

    #[test]
    fn parses_mem_command() {
        let parsed = parse_command(b"mem");
        assert_eq!(parsed.kind, CommandKind::Mem);
        assert_eq!(parsed.arg, b"");
    }

    #[test]
    fn parses_ls_default_path() {
        let parsed = parse_command(b"ls");
        assert_eq!(parsed.kind, CommandKind::Ls);
        assert_eq!(parsed.arg, b"/");
    }

    #[test]
    fn parses_cat_path() {
        let parsed = parse_command(b"cat /motd.txt");
        assert_eq!(parsed.kind, CommandKind::Cat);
        assert_eq!(parsed.arg, b"/motd.txt");
    }

    #[test]
    fn parses_stat_path() {
        let parsed = parse_command(b"stat /version.txt");
        assert_eq!(parsed.kind, CommandKind::Stat);
        assert_eq!(parsed.arg, b"/version.txt");
    }

    #[test]
    fn normalizes_relative_path() {
        assert_eq!(normalize_simplefs_path("motd.txt"), "/motd.txt");
    }

    #[test]
    fn keeps_absolute_path() {
        assert_eq!(normalize_simplefs_path("/motd.txt"), "/motd.txt");
    }

    #[test]
    fn parses_empty_line() {
        let parsed = parse_command(b"");
        assert_eq!(parsed.kind, CommandKind::Empty);
        assert_eq!(parsed.arg, b"");
    }
}
