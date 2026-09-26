//! A bounded window of the recent past, addressed by absolute index. [M20 P4]
//!
//! The thing a live session records *into* while nothing has happened yet. A ball strike is only
//! recognisable once it has been heard, and by then the five seconds of address that make the
//! swing measurable are already past — so capture runs continuously into a fixed-size buffer and
//! [`crate::clip`] cuts backwards out of it.
//!
//! # Absolute indices, not offsets into the buffer
//!
//! Every index in and out of this type counts from the start of the *stream*, not from the start
//! of what is still held. That is the whole design: [`crate::stream::Trigger::sample`] and
//! [`crate::strike::Strike::sample`] are already absolute, so a clip span computed from a trigger
//! can be handed straight to [`Ring::copy`] with no arithmetic in between — and arithmetic in
//! between is where an off-by-one-hop becomes a clip cut 5 ms from where it was asked for.
//!
//! # It refuses rather than guessing
//!
//! [`Ring::copy`] returns `None` for a span that has already been overwritten. A ring buffer that
//! silently returned the oldest data it still had would hand back a clip of the wrong moment, and
//! the caller would have no way to tell — the same reason a checkpoint that cannot be measured
//! returns `None` rather than a zero (ADR-010 §2). Sizing is [`crate::clip`]'s job and it sizes
//! for the longest clip that module can produce, so this is a wiring failure and not a data
//! condition; it is reported rather than raised because the caller (a capture loop) is better
//! placed to decide whether to drop the clip or stop the session.
//!
//! # Generic over the item
//!
//! `i16` audio samples today. ADR-030 §1 gives Rust "camera capture, the ring buffer, clip
//! cutting" as one job, and ROADMAP §M21 asks for "a ring buffer with host-clock timestamps" on
//! the video side; that is the named second implementation, and it holds encoded frames rather
//! than samples. What the two share is exactly what is written here — a capacity, a base index
//! and a refusal — so the video edge instantiates this rather than writing it again.

use std::collections::VecDeque;

/// A fixed-capacity buffer of the most recent items, indexed from the start of the stream.
#[derive(Debug)]
pub struct Ring<T> {
    items: VecDeque<T>,
    capacity: usize,
    /// Absolute index of `items[0]`. Rises as the oldest items are dropped.
    base: usize,
}

impl<T: Copy> Ring<T> {
    /// A ring that holds `capacity` items.
    ///
    /// # Panics
    /// If `capacity` is zero — a ring that holds nothing is a wiring mistake at construction, and
    /// finding out about it at the first `copy` would be finding out about it during a swing.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "a ring buffer must hold something");
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
            base: 0,
        }
    }

    /// Append items, dropping whatever no longer fits.
    pub fn extend(&mut self, items: &[T]) {
        // A block larger than the whole ring keeps only its tail. Not a case any capture device
        // produces, but the arithmetic below is only correct if it is handled, and "the caller
        // will not do that" is not a property this type can check.
        let keep = items.len().min(self.capacity);
        self.base += items.len() - keep;
        self.items.extend(&items[items.len() - keep..]);
        let overflow = self.items.len().saturating_sub(self.capacity);
        if overflow > 0 {
            self.items.drain(..overflow);
            self.base += overflow;
        }
    }

    /// The lowest absolute index still held.
    pub fn first(&self) -> usize {
        self.base
    }

    /// One past the highest absolute index held — equivalently, how many items the stream has
    /// delivered so far.
    pub fn position(&self) -> usize {
        self.base + self.items.len()
    }

    /// How many items are held.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many items this ring can hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The items in `[start, end)`, or `None` if any of that span is gone or not yet arrived.
    ///
    /// Copies rather than lending two slices. The span is a whole clip, so this runs once per
    /// shot and costs one allocation of a few hundred kilobytes; a split-slice API would push
    /// the wrap-around into every caller in exchange for nothing measurable.
    pub fn copy(&self, start: usize, end: usize) -> Option<Vec<T>> {
        if end < start || start < self.base || end > self.position() {
            return None;
        }
        Some(
            self.items
                .iter()
                .skip(start - self.base)
                .take(end - start)
                .copied()
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_went_in_comes_back_out_at_the_index_it_went_in_at() {
        let mut ring = Ring::with_capacity(100);
        ring.extend(&(0i32..40).collect::<Vec<_>>());
        ring.extend(&(40i32..80).collect::<Vec<_>>());
        assert_eq!(ring.copy(30, 50).unwrap(), (30i32..50).collect::<Vec<_>>());
        assert_eq!(ring.position(), 80);
        assert_eq!(ring.first(), 0);
    }

    #[test]
    fn the_oldest_items_are_dropped_and_the_indices_do_not_move() {
        let mut ring = Ring::with_capacity(10);
        ring.extend(&(0i32..25).collect::<Vec<_>>());
        assert_eq!(ring.first(), 15);
        assert_eq!(ring.position(), 25);
        assert_eq!(ring.copy(15, 25).unwrap(), (15i32..25).collect::<Vec<_>>());
    }

    /// The property the whole type exists for: a span that is gone is reported gone.
    #[test]
    fn a_span_that_has_been_overwritten_is_refused_rather_than_approximated() {
        let mut ring = Ring::with_capacity(10);
        ring.extend(&(0i32..25).collect::<Vec<_>>());
        assert!(
            ring.copy(10, 20).is_none(),
            "14 is gone; 15-19 is not enough"
        );
        assert!(ring.copy(20, 30).is_none(), "25-29 has not happened yet");
        assert!(ring.copy(20, 25).is_some());
    }

    /// A block bigger than the ring keeps its tail, and `first` says so.
    #[test]
    fn a_block_larger_than_the_ring_keeps_its_tail() {
        let mut ring = Ring::with_capacity(4);
        ring.extend(&(0i32..10).collect::<Vec<_>>());
        assert_eq!(ring.first(), 6);
        assert_eq!(ring.copy(6, 10).unwrap(), vec![6, 7, 8, 9]);
    }

    /// How the caller chops up the stream changes nothing — the same property
    /// [`crate::stream`] asserts about the detector, for the same reason.
    #[test]
    fn the_block_size_does_not_change_what_is_held() {
        let data: Vec<i32> = (0..500).collect();
        for block in [1, 7, 64, 500] {
            let mut ring = Ring::with_capacity(100);
            for chunk in data.chunks(block) {
                ring.extend(chunk);
            }
            assert_eq!(ring.first(), 400, "block size {block}");
            assert_eq!(
                ring.copy(400, 500).unwrap(),
                (400i32..500).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn an_empty_span_is_empty_and_not_an_error() {
        let mut ring = Ring::with_capacity(10);
        ring.extend(&[1i32, 2, 3]);
        assert_eq!(ring.copy(2, 2).unwrap(), Vec::<i32>::new());
    }
}
