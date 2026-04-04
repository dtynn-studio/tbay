use std::ops::Deref;

#[derive(Clone)]
pub struct RingBuffer<T: Copy> {
    inner: Vec<T>,
    capacity: usize,
    added: usize,
}

impl<T: Copy> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        RingBuffer {
            inner: Vec::with_capacity(capacity),
            capacity,
            added: 0,
        }
    }

    pub fn update(&mut self, item: T) -> Option<T> {
        if self.added < self.capacity {
            self.inner.push(item);
            self.added += 1;
            None
        } else {
            let slot = self.added % self.capacity;
            self.added += 1;
            let replaced = std::mem::replace(&mut self.inner[slot], item);
            Some(replaced)
        }
    }

    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.inner.len() {
            None
        } else {
            let slot = if self.added <= self.capacity {
                index
            } else {
                (self.added - self.capacity + index) % self.capacity
            };
            Some(self.inner[slot])
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.added >= self.capacity
    }
}

impl<T: Copy> Deref for RingBuffer<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
