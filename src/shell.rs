use crate::{arch, console};
use core::arch::asm;

const MAX_LINE: usize = 128;
const HELP_TEXT: &[u8] = b"commands: help echo clear panic halt reboot";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    Empty,
    Help,
    Echo,
    Clear,
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

    console::write_line(b"Type 'help' for commands.");
    prompt();

    loop {
        if let Some(ch) = arch::x86_64::keyboard::try_read_char() {
            match ch {
                b'\n' => {
                    console::write_byte(b'\n');
                    execute_command(&line_buf[..len]);
                    len = 0;
                    prompt();
                }
                8 => {
                    if len > 0 {
                        len -= 1;
                        console::backspace();
                    }
                }
                b if is_printable_ascii(b) => {
                    if len < MAX_LINE {
                        line_buf[len] = b;
                        len += 1;
                        console::write_byte(b);
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

fn execute_command(line: &[u8]) {
    let parsed = parse_command(line);

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
    fn parses_empty_line() {
        let parsed = parse_command(b"");
        assert_eq!(parsed.kind, CommandKind::Empty);
        assert_eq!(parsed.arg, b"");
    }
}
