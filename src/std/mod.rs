//! Items required for running in `std` environments.

#[cfg(target_os = "linux")]
mod io_uring;
#[cfg(unix)]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

use std::{
    sync::Arc,
    task::Wake,
    thread::{self, Thread},
};

#[cfg(target_os = "windows")]
pub use self::windows::{ethercat_now, tx_rx_task, tx_rx_task_blocking, TxRxTaskConfig};
#[cfg(unix)]
pub use unix::{ethercat_now, tx_rx_task};
// io_uring is Linux-only
#[cfg(target_os = "linux")]
pub use io_uring::tx_rx_task_io_uring;

struct ParkSignal {
    current_thread: Thread,
}

impl ParkSignal {
    fn new() -> Self {
        Self {
            current_thread: thread::current(),
        }
    }

    fn wait(&self) {
        thread::park();
    }

    // fn wait_timeout(&self, timeout: Duration) {
    //     thread::park_timeout(timeout)
    // }
}

impl Wake for ParkSignal {
    fn wake(self: Arc<Self>) {
        self.current_thread.unpark();
    }
}
