//! Ansteuerung des Programmable Interval Timer (PIT).
//!
//! Hintergrund: <https://wiki.osdev.org/Programmable_Interval_Timer>

use core::sync::atomic::{AtomicU64, Ordering};

use super::io::outb;

const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_BASE_FREQUENCY: u32 = 1_193_182;
const PIT_TARGET_HZ: u32 = 100;

static TICKS: AtomicU64 = AtomicU64::new(0);

/// Programmiert PIT Kanal 0 auf `PIT_TARGET_HZ`.
pub fn init() {
    let divisor = (PIT_BASE_FREQUENCY / PIT_TARGET_HZ) as u16;
    outb(PIT_COMMAND, 0x36);
    outb(PIT_CHANNEL0_DATA, (divisor & 0xff) as u8);
    outb(PIT_CHANNEL0_DATA, (divisor >> 8) as u8);
}

/// Wird bei jedem Timer-IRQ aufgerufen und erhöht den Tickzähler.
pub fn on_tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Liefert die seit Start gezählten PIT-Ticks.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::PIT_TARGET_HZ;

    #[test]
    fn pit_target_frequency_is_reasonable() {
        assert!(PIT_TARGET_HZ >= 10);
        assert!(PIT_TARGET_HZ <= 1000);
    }
}
