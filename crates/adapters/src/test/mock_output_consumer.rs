use crate::{transport::Step, OutputConsumer};
use std::sync::{Arc, Mutex};

pub struct MockOutputConsumer {
    pub data: Arc<Mutex<Vec<u8>>>,
    max_buffer_size_bytes: usize,
}

impl Default for MockOutputConsumer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockOutputConsumer {
    pub fn new() -> Self {
        Self::with_max_buffer_size_bytes(usize::MAX)
    }

    pub fn with_max_buffer_size_bytes(bytes: usize) -> Self {
        Self {
            data: Arc::new(Mutex::new(Vec::new())),
            max_buffer_size_bytes: bytes,
        }
    }
}

impl OutputConsumer for MockOutputConsumer {
    fn max_buffer_size_bytes(&self) -> usize {
        self.max_buffer_size_bytes
    }

    fn batch_start(&mut self, _step: Step) {}
    fn push_buffer(&mut self, buffer: &[u8]) {
        self.data.lock().unwrap().extend_from_slice(buffer)
    }
    fn batch_end(&mut self) {}
}
