pub struct ReadBuffer {
    read_buffer: Vec<u8>,
    start_pos: usize,
    end_pos: usize,
}

impl ReadBuffer {
    pub fn new(buffer_size: usize) -> Self {
        let mut read_buffer = Vec::with_capacity(buffer_size);

        unsafe {
            read_buffer.set_len(buffer_size);

            Self {
                read_buffer,
                start_pos: 0,
                end_pos: 0,
            }
        }
    }

    pub fn get_write_buf(&mut self) -> Option<&mut [u8]> {
        if self.end_pos == self.read_buffer.len() {
            return None;
        }

        Some(&mut self.read_buffer[self.end_pos..])
    }

    pub fn advance(&mut self, size: usize) {
        self.end_pos += size;
    }

    pub fn get_ready_to_read(&self) -> &[u8] {
        &self.read_buffer[self.start_pos..self.end_pos]
    }

    pub fn write_to_buffer(&mut self, out: &mut [u8]) -> usize {
        let available_to_write = self.end_pos - self.start_pos;

        if available_to_write == 0 {
            return 0;
        }

        let to_write_size = if available_to_write < out.len() {
            let buffer_to_copy = &self.read_buffer[self.start_pos..self.end_pos];

            let out = &mut out[..available_to_write];
            out.copy_from_slice(buffer_to_copy);
            available_to_write
        } else {
            let buffer_to_copy = &self.read_buffer[self.start_pos..self.start_pos + out.len()];
            out.copy_from_slice(buffer_to_copy);
            out.len()
        };

        self.start_pos += to_write_size;

        if self.start_pos == self.end_pos {
            self.start_pos = 0;
            self.end_pos = 0;
        }

        to_write_size
    }
}
