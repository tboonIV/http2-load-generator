use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;

pub struct ApiStats {
    success_counter: AtomicU32,
    error_counter: AtomicU32,
    total_rtt: AtomicU64,
    total_retry: AtomicU32,
}

unsafe impl Sync for ApiStats {}
unsafe impl Send for ApiStats {}

impl ApiStats {
    pub fn new() -> ApiStats {
        ApiStats {
            success_counter: AtomicU32::new(0),
            error_counter: AtomicU32::new(0),
            total_rtt: AtomicU64::new(0),
            total_retry: AtomicU32::new(0),
        }
    }

    pub fn inc_success(&self) {
        self.success_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_success(&self) -> u32 {
        self.success_counter
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn inc_error(&self) {
        self.error_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_error(&self) -> u32 {
        self.error_counter
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn inc_rtt(&self, rtt: u64) {
        self.total_rtt
            .fetch_add(rtt, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_rtt(&self) -> u64 {
        self.total_rtt.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn inc_retry(&self, retry: u32) {
        self.total_retry
            .fetch_add(retry, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_retry(&self) -> u32 {
        self.total_retry.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// pub struct RequestStats {
//     success_counter: u32,
//     error_counter: u32,
//     total_rtt: u64,
//     total_retry: u32,
// }
//
// impl RequestStats {
//     pub fn new() -> RequestStats {
//         RequestStats {
//             success_counter: 0,
//             error_counter: 0,
//             total_rtt: 0,
//             total_retry: 0,
//         }
//     }
//
//     pub fn inc_success(&mut self) {
//         self.success_counter += 1;
//     }
//
//     pub fn get_success(&self) -> u32 {
//         self.success_counter
//     }
//
//     pub fn inc_error(&mut self) {
//         self.error_counter += 1;
//     }
//
//     pub fn get_error(&self) -> u32 {
//         self.error_counter
//     }
//
//     pub fn inc_rtt(&mut self) {
//         self.total_rtt += 1;
//     }
//
//     pub fn get_rtt(&self) -> u64 {
//         self.total_rtt
//     }
//
//     pub fn inc_retry(&mut self) {
//         self.total_retry += 1;
//     }
//
//     pub fn get_retry(&self) -> u32 {
//         self.total_retry
//     }
// }
