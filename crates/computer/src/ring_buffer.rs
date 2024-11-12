use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    length: usize,
    inner: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    pub fn new(length: usize) -> Self {
        assert!(length > 0, "ring buffer must have non-zero length");

        Self {
            length,
            inner: VecDeque::with_capacity(length),
        }
    }

    pub fn push_back(&mut self, sample: T) {
        while self.inner.len() >= self.length {
            self.inner.pop_front();
        }

        self.inner.push_back(sample);
    }

    pub fn capacity(&self) -> usize {
        self.length
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_full(&self) -> bool {
        self.inner.len() == self.capacity()
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.inner.iter()
    }
}
