use std::collections::VecDeque;

/// Un-growable ring buffer backed by VecDeque.
/// When full, pushed-back elements pop out elements from the front.
#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    capacity: usize,
    inner: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    /// Construct new RingBuffer with given capacity. Panic when capacity is 0.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ring buffer must have non-zero capacity");

        Self {
            capacity,
            inner: VecDeque::with_capacity(capacity),
        }
    }

    /// Push a single element to the back of the buffer. If the buffer is full, pop the front-most
    /// element and return it.
    pub fn push_back(&mut self, sample: T) -> Option<T> {
        if self.inner.len() < self.capacity {
            self.inner.push_back(sample);
            return None;
        }

        let front_element = self
            .inner
            .pop_front()
            .expect("self.capacity can't be 0 and we checked we are full");

        self.inner.push_back(sample);

        Some(front_element)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.inner.len() == self.capacity
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Resize the buffer. When downsizing elements may be popped from the front.
    pub fn set_capacity(&mut self, capacity: usize) {
        assert!(capacity > 0, "ring buffer must have non-zero capacity");

        self.capacity = capacity;
        while self.inner.len() > self.capacity {
            self.inner.pop_front();
        }
    }
}
