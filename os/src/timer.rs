use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

pub fn get_time() -> usize {
    time::read()
}

const TICKET_PER_SECOND: usize = 100;

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKET_PER_SECOND);
}

const MSEC_PER_SEC: usize = 1000;

pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}