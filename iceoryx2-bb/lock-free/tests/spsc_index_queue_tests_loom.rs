// Copyright (c) 2024 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(all(loom, test))]

use std::sync::atomic::Ordering;

use loom::sync::{Arc, Notify};
use loom::thread;

// Re-export the SPSC index queue with loom support
use iceoryx2_bb_lock_free::spsc::index_queue::*;

#[test]
fn spsc_index_queue_loom_tests() {
    loom::model(|| {
        const CAPACITY: usize = 4;
        let queue = Arc::new(FixedSizeIndexQueue::<CAPACITY>::new());
        let notify = Arc::new(OneShotSignal::new());

        let queue_producer = Arc::clone(&queue);
        let notify_producer = Arc::clone(&notify);

        let queue_consumer = Arc::clone(&queue);
        let notify_consumer = Arc::clone(&notify);

        let producer = thread::spawn(move || {
            let mut producer = queue_producer.acquire_producer().unwrap();
            assert!(producer.push(42));
            notify_producer.signal();
        });

        let consumer = thread::spawn(move || {
            notify_consumer.wait();
            let mut consumer = queue_consumer.acquire_consumer().unwrap();
            assert_eq!(consumer.pop().unwrap(), 42);
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    });
}

use loom::sync::atomic::AtomicBool;

struct OneShotSignal {
    ready: AtomicBool,
    notify: Notify,
}

impl OneShotSignal {
    fn new() -> Self {
        Self {
            ready: AtomicBool::new(false),
            notify: Notify::new(),
        }
    }

    /// Producer side: mark it ready and wake consumer.
    fn signal(&self) {
        // Publish "ready" with release semantics
        self.ready.store(true, Ordering::Release);
        self.notify.notify();
    }

    /// Consumer side: wait until `ready == true`.
    fn wait(&self) {
        loop {
            // Fast path: already signaled?
            if self.ready.load(Ordering::Acquire) {
                return;
            }

            // Slow path: sleep until someone notifies us.
            // Spurious wakeups are fine; we loop and re-check.
            self.notify.wait();
        }
    }
}
