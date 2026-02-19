use crate::{arch, console};
use alloc::vec::Vec;
use core::arch::asm;

const MAX_LINE: usize = 128;
const MAX_HISTORY: usize = 16;
const HELP_TEXT: &[u8] = b"commands: help echo clear history mem ticks panic halt reboot";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    Empty,
    Help,
    Echo,
    Clear,
    History,
    Mem,
    Ticks,
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

fn parse_command(line: &[u8]) -> ParsedCommand<'_> {
    if line.is_empty() {
        return ParsedCommand {
            kind: CommandKind::Empty,
            arg: b"",
        };
    }

    if line == b"help" {
        return ParsedCommand {
            kind: CommandKind::Help,
            arg: b"",
        };
    }

    if line == b"clear" {
        return ParsedCommand {
            kind: CommandKind::Clear,
            arg: b"",
        };
    }

    if line == b"history" {
        return ParsedCommand {
            kind: CommandKind::History,
            arg: b"",
        };
    }

    if line == b"mem" {
        return ParsedCommand {
            kind: CommandKind::Mem,
            arg: b"",
        };
    }

    if line == b"ticks" {
        return ParsedCommand {
            kind: CommandKind::Ticks,
            arg: b"",
        };
    }

    if line == b"panic" {
        return ParsedCommand {
            kind: CommandKind::Panic,
            arg: b"",
        };
    }

    if line == b"halt" {
        return ParsedCommand {
            kind: CommandKind::Halt,
            arg: b"",
        };
    }

    if line == b"reboot" {
        return ParsedCommand {
            kind: CommandKind::Reboot,
            arg: b"",
        };
    }

    if let Some(rest) = line.strip_prefix(b"echo ") {
        return ParsedCommand {
            kind: CommandKind::Echo,
            arg: rest,
        };
    }

    ParsedCommand {
        kind: CommandKind::Unknown,
        arg: b"",
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
    ok &= check_parse(b"echo", CommandKind::Unknown, b"");
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
    use super::{parse_command, CommandKind};

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
    fn parses_empty_line() {
        let parsed = parse_command(b"");
        assert_eq!(parsed.kind, CommandKind::Empty);
        assert_eq!(parsed.arg, b"");
    }
}
