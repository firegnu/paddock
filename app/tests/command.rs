//! Concurrent command calls must not keep each other's pipes open.
use paddock::command;
use std::{
    sync::{
        Barrier,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[test]
fn a_concurrent_long_lived_child_does_not_hold_another_calls_pipes() {
    // Repeat the overlap: on macOS the interval between pipe() and CLOEXEC is tiny.
    for round in 0..512 {
        let ready = Barrier::new(9);
        let stop = AtomicBool::new(false);
        let output = thread::scope(|scope| {
            let slow: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        ready.wait();
                        command::run("/bin/sleep", &["30"], None, Duration::from_secs(5), &stop)
                    })
                })
                .collect();
            ready.wait();
            let started = Instant::now();
            let output = command::run_with_input(
                "/bin/cat",
                &[],
                None,
                &[],
                b"independent input and output\n".to_vec(),
                Duration::from_secs(1),
                &AtomicBool::new(false),
            );
            let elapsed = started.elapsed();
            stop.store(true, Ordering::Relaxed);
            for slow in slow {
                assert!(slow.join().unwrap().is_err());
            }
            assert!(
                elapsed < Duration::from_secs(1),
                "round {round}: {elapsed:?}"
            );
            output
        });
        let output = output.unwrap_or_else(|error| panic!("round {round}: {error:#}"));
        assert!(output.status.success());
        assert_eq!(output.stdout, b"independent input and output\n");
        assert!(output.stderr.is_empty());
    }
}
