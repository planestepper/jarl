use std::time::{SystemTime, UNIX_EPOCH};
use ::bounded_vec_deque::BoundedVecDeque;
use clap::Parser;


pub struct Keeper {
    limit: u32,
    period_in_secs: f64,
    queue: BoundedVecDeque<f64>,
    backoff_count: f32,
    base_delay: f32,
}

impl Keeper {
    
    pub fn new(limit: u32, period: u32) -> Self {
        assert!(limit > 0, "Max requests per period must be greater than 0.");
        assert!(period > 0, "Period must be greater than 0.");

        Keeper {
            limit,
            period_in_secs: period as f64,
            queue: BoundedVecDeque::new((limit + 1) as usize),
            backoff_count: 0.0,
            base_delay: (period as f32 / limit as f32).max(0.01),
        }
    }

    pub fn get_delay(&mut self) -> f32 {
        let time_since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time error");

        let timestamp = time_since_epoch.as_secs() as f64 + 
                             time_since_epoch.subsec_millis() as f64 * 0.001;
        self.queue.push_back(timestamp);

        if self.queue.len() == (self.limit + 1) as usize {
            let last = self.queue.pop_front().unwrap_or_default();
            let diff = timestamp - last;


            if diff < self.period_in_secs {
                let adjustment = (self.period_in_secs - diff) as f32;
                self.backoff_count += 1.0;

                return self.base_delay * self.backoff_count + adjustment;
            }
        }

        self.backoff_count = 0.0;
        0.0
    }

}


#[derive(Parser)]
pub struct Cli {
    /// Name of the service to rate-limit, not used by the code,
    /// serving as a CLI reference only
    #[arg(long)]
    service: String,
    
    /// Maximum number of requests to allow within the period
    #[arg(long)]
    pub requests: u32,
    
    /// Period to enforce rate over, in seconds
    #[arg(long)]
    pub period: u32,
    
    /// IPv4 interface to bind to, normally 0.0.0.0
    #[arg(long)]
    pub ip: String,

    /// Port to bind to
    #[arg(long)]
    pub port: u32,
}


// Unit tests
#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use crate::Keeper;

    #[test]
    /// The base delay is the maximum value between the expected average time for each
    /// request within the period (max requests / period) and 0.01.
    fn base_delay_values() {
        let keeper_1 = Keeper::new(1, 1);
        assert_eq!(keeper_1.base_delay, 1.0);

        let keeper_2 = Keeper::new(10, 1);
        assert_eq!(keeper_2.base_delay, 0.1);

        let keeper_3 = Keeper::new(10000, 1);
        assert_eq!(keeper_3.base_delay, 0.01);
    }

    #[test]
    #[should_panic]
    fn reject_period_of_zero() {
        let _keeper = Keeper::new(1, 0);
    }

    #[test]
    #[should_panic]
    fn reject_max_zero_requests() {
        let _keeper = Keeper::new(0, 1);
    }

    #[test]
    /// Ensure functionality for one request per second scenario
    fn minimum_rate() {
        let mut keeper = Keeper::new(1, 1);

        keeper.get_delay();
        // Expect second request within a second to return some delay > 0
        let delay_1 = keeper.get_delay();
        assert!(delay_1 > 0.0, "Delay should be greater than 0.");

        // By waiting the delay, Keeper should reset after a new get_delay call
        sleep(Duration::from_millis((delay_1 * 1000.0) as u64));
        let delay_2 = keeper.get_delay();
        assert!(keeper.backoff_count == 0.0, "Backoff count should have reset.");

        // After the reset, the delay returned should be 0
        assert!(delay_2 == 0.0, "Delay should be 0 after a reset.");
    }

    #[test]
    /// Ensure functionality for a more common scenario (requests > 1 and period > 1)
    fn normal_rate() {
        let mut keeper = Keeper::new(100, 5);

        for _ in 0..100 {
            assert!(keeper.get_delay() == 0.0, "Delay for requests within rate limit should be 0.");
        }
        // Expect 100th request within period to return some delay > 0
        let delay_1 = keeper.get_delay();
        assert!(delay_1 > 0.0, "Delay should be greater than 0.");

        // By waiting the delay, Keeper should reset after a new get_delay call
        sleep(Duration::from_millis((delay_1 * 1000.0) as u64));
        let delay_2 = keeper.get_delay();
        assert!(keeper.backoff_count == 0.0, "Backoff count should have reset.");

        // After the reset, the delay returned should be 0
        assert!(delay_2 == 0.0, "Delay should be 0 after a reset.");
    }


}
