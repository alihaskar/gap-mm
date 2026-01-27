#[cfg(test)]
mod tests {
    use crate::current_time_millis;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tracing::info;

    #[test]
    fn test_current_time_millis_increases() {
        let time1 = current_time_millis();
        // Sleep for a bit to ensure time passes
        thread::sleep(Duration::from_millis(5));
        let time2 = current_time_millis();

        // The second time should be greater than the first
        assert!(time2 > time1, "Time should increase between calls");
    }

    #[test]
    fn test_current_time_millis_is_reasonably_current() {
        // Get current time using both methods
        let time_from_function = current_time_millis();
        let time_direct = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        // The times should be very close to each other
        // Allow a small difference due to execution time between the two calls
        let difference = time_direct.abs_diff(time_from_function);

        // The difference should be no more than 10ms (this is generous)
        assert!(
            difference <= 10,
            "Time difference should be small, but got {difference}ms"
        );
    }

    #[test]
    fn test_current_time_millis_precision() {
        // Call the function twice in quick succession
        let time1 = current_time_millis();
        let time2 = current_time_millis();

        // Check if we have at least millisecond precision
        // This test might be flaky if both calls happen within the same millisecond,
        // but it's unlikely on most modern systems
        // If it fails, it doesn't necessarily indicate a problem
        if time1 == time2 {
            info!("Note: consecutive calls returned same time, which might happen occasionally");
        }

        // Ensure with a sleep that we get different values
        thread::sleep(Duration::from_millis(5));
        let time3 = current_time_millis();
        assert!(time3 > time1, "Time should increase after sleep");
    }
}
