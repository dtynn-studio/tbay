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

    pub fn slot(&self, i: usize) -> Option<usize> {
        if i >= self.inner.len() {
            return None;
        }

        Some(self.slot_inner(i))
    }

    fn slot_inner(&self, i: usize) -> usize {
        if self.added <= self.capacity {
            i
        } else {
            (self.added - self.capacity + i) % self.capacity
        }
    }

    pub fn get(&self, index: usize) -> Option<T> {
        self.slot(index).map(|slot| self.inner[slot])
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.added >= self.capacity
    }

    pub fn size(&self) -> usize {
        if self.added < self.capacity {
            self.added
        } else {
            self.capacity
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &T> {
        let last = self.added.min(self.capacity);
        let items = &self.inner[..last];
        let idx0 = self.slot_inner(0);
        let (left, right) = items.split_at(idx0);
        right.iter().chain(left)
    }

    pub fn last(&self) -> Option<&T> {
        if self.added <= self.capacity {
            self.inner.last()
        } else {
            Some(&self.inner[self.slot_inner(self.capacity - 1)])
        }
    }
}

impl<T: Copy> Deref for RingBuffer<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        if self.is_full() {
            &self.inner
        } else {
            &self.inner[..self.added]
        }
    }
}

#[cfg(test)]
mod tests;
