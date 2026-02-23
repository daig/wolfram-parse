//! A byte buffer that can return the current byte and advance to the next byte.

use crate::read::Reader;

impl Reader<'_> {
    /// Return current byte and advance to the next byte.
    ///
    /// Precondition: buffer is pointing to current byte
    /// Postcondition: buffer is pointing to 1 byte past current byte
    pub(crate) fn next_byte(&mut self) -> u8 {
        if self.offset >= self.input.len() {
            self.was_eof = true;
            // TODO: Make this return None.
            return 0xff;
        }

        let byte = self.buffer()[0];

        self.offset += 1;

        return byte;
    }

    pub(crate) fn current_byte(&self) -> u8 {
        if self.offset >= self.input.len() {
            return 0xff;
        }

        return self.buffer()[0];
    }
}
