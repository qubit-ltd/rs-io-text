// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Allocation tracking used by the owned string-adapter benchmark.

use std::alloc::{
    GlobalAlloc,
    Layout,
    System,
};
use std::sync::atomic::{
    AtomicBool,
    AtomicUsize,
    Ordering,
};

static ENABLED: AtomicBool = AtomicBool::new(false);
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// A system allocator that can report peak live bytes for one operation.
pub struct TrackingAllocator;

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

impl TrackingAllocator {
    fn add(size: usize) {
        if !ENABLED.load(Ordering::Relaxed) {
            return;
        }
        let current = CURRENT.fetch_add(size, Ordering::Relaxed) + size;
        PEAK.fetch_max(current, Ordering::Relaxed);
    }

    fn subtract(size: usize) {
        if !ENABLED.load(Ordering::Relaxed) {
            return;
        }
        let _ = CURRENT.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |current| Some(current.saturating_sub(size)),
        );
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The request is forwarded unchanged to the system allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            Self::add(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The request is forwarded unchanged to the system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            Self::add(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        Self::subtract(layout.size());
        // SAFETY: The pointer and layout came from this allocator, which
        // delegates all allocation requests to the system allocator.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(
        &self,
        pointer: *mut u8,
        old_layout: Layout,
        new_size: usize,
    ) -> *mut u8 {
        // SAFETY: The request is forwarded unchanged to the system allocator.
        let new_pointer =
            unsafe { System.realloc(pointer, old_layout, new_size) };
        if !new_pointer.is_null() && ENABLED.load(Ordering::Relaxed) {
            if new_size >= old_layout.size() {
                Self::add(new_size - old_layout.size());
            } else {
                Self::subtract(old_layout.size() - new_size);
            }
        }
        new_pointer
    }
}

/// Runs one single-threaded operation and returns its peak live allocation.
pub fn measure_peak<F, T>(operation: F) -> (T, usize)
where
    F: FnOnce() -> T,
{
    CURRENT.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
    ENABLED.store(true, Ordering::Relaxed);
    let result = operation();
    ENABLED.store(false, Ordering::Relaxed);
    let peak = PEAK.load(Ordering::Relaxed);
    (result, peak)
}
