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
        Keeper {
            limit,
            period_in_secs: period as f64,
            queue: BoundedVecDeque::new(limit as usize),
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

        if self.queue.len() >= self.limit as usize {
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