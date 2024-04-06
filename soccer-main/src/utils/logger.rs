use core::{
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use critical_section::RestoreState;
use defmt::Encoder;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

const BUF_SIZE: usize = 4096;

pub static LOGGER_CHANNEL: Channel<CriticalSectionRawMutex, u8, BUF_SIZE> = Channel::new();

#[defmt::global_logger]
struct Logger;

static TAKEN: AtomicBool = AtomicBool::new(false);
static mut CS_RESTORE: RestoreState = RestoreState::invalid();
static mut ENCODER: Encoder = Encoder::new();

unsafe impl defmt::Logger for Logger {
    fn acquire() {
        let restore = unsafe { critical_section::acquire() };

        (!TAKEN.load(Ordering::Relaxed)).then_some(0).unwrap();
        TAKEN.store(true, Ordering::Relaxed);

        unsafe { CS_RESTORE = restore };
        unsafe { ENCODER.start_frame(do_write) };
    }

    unsafe fn flush() {
        handle().flush();
    }

    unsafe fn release() {
        ENCODER.end_frame(do_write);

        TAKEN.store(false, Ordering::Relaxed);

        let restore = CS_RESTORE;

        critical_section::release(restore);
    }

    unsafe fn write(bytes: &[u8]) {
        ENCODER.write(bytes, do_write);
    }
}

fn do_write(bytes: &[u8]) {
    unsafe {
        handle().write_all(bytes);
        let _ = LOGGER_CHANNEL.try_send(bytes[0]);
    }
}

#[repr(C)]
struct Header {
    id: [u8; 16],
    max_up_channels: usize,
    max_down_channels: usize,
    up_channel: UpChannel,
}

const MODE_MASK: usize = 0b11;
const MODE_BLOCK_IF_FULL: usize = 2;
const MODE_NON_BLOCKING_TRIM: usize = 1;

unsafe fn handle() -> &'static UpChannel {
    #[no_mangle]
    static mut _SEGGER_RTT: Header = Header {
        id: *b"SEGGER RTT\0\0\0\0\0\0",
        max_up_channels: 1,
        max_down_channels: 0,
        up_channel: UpChannel {
            name: &NAME as *const _ as *const u8,
            buffer: unsafe { &mut BUFFER as *mut _ as *mut u8 },
            size: BUF_SIZE,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            flags: AtomicUsize::new(MODE_NON_BLOCKING_TRIM),
        },
    };

    #[link_section = ".uninit.defmt-rtt.BUFFER"]
    static mut BUFFER: [u8; BUF_SIZE] = [0; BUF_SIZE];

    #[link_section = ".data"]
    static NAME: [u8; 6] = *b"defmt\0";

    &_SEGGER_RTT.up_channel
}

#[repr(C)]
struct UpChannel {
    pub name: *const u8,
    pub buffer: *mut u8,
    pub size: usize,
    pub write: AtomicUsize,
    pub read: AtomicUsize,
    pub flags: AtomicUsize,
}

impl UpChannel {
    fn write_all(&self, mut bytes: &[u8]) {
        let write = match self.host_is_connected() {
            true => Self::blocking_write,
            false => Self::nonblocking_write,
        };

        while !bytes.is_empty() {
            let consumed = write(self, bytes);
            if consumed != 0 {
                bytes = &bytes[consumed..];
            }
        }
    }

    fn blocking_write(&self, bytes: &[u8]) -> usize {
        if bytes.is_empty() {
            return 0;
        }

        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Acquire);
        let available = available_buffer_size(read, write);

        if available == 0 {
            return 0;
        }

        self.write_impl(bytes, write, available)
    }

    fn nonblocking_write(&self, bytes: &[u8]) -> usize {
        let write = self.write.load(Ordering::Acquire);

        self.write_impl(bytes, write, BUF_SIZE)
    }

    fn write_impl(&self, bytes: &[u8], cursor: usize, available: usize) -> usize {
        let len = bytes.len().min(available);

        unsafe {
            if cursor + len > BUF_SIZE {
                let pivot = BUF_SIZE - cursor;
                ptr::copy_nonoverlapping(bytes.as_ptr(), self.buffer.add(cursor), pivot);
                ptr::copy_nonoverlapping(bytes.as_ptr().add(pivot), self.buffer, len - pivot);
            } else {
                ptr::copy_nonoverlapping(bytes.as_ptr(), self.buffer.add(cursor), len);
            }
        }

        self.write
            .store(cursor.wrapping_add(len) % BUF_SIZE, Ordering::Release);

        len
    }

    pub fn flush(&self) {
        if !self.host_is_connected() {
            return;
        }

        let read = || self.read.load(Ordering::Relaxed);
        let write = || self.write.load(Ordering::Relaxed);
        while read() != write() {}
    }

    fn host_is_connected(&self) -> bool {
        self.flags.load(Ordering::Relaxed) & MODE_MASK == MODE_BLOCK_IF_FULL
    }
}

fn available_buffer_size(read_cursor: usize, write_cursor: usize) -> usize {
    if read_cursor > write_cursor {
        read_cursor - write_cursor - 1
    } else if read_cursor == 0 {
        BUF_SIZE - write_cursor - 1
    } else {
        BUF_SIZE - write_cursor
    }
}
