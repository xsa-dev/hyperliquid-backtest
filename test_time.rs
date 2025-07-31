use chrono::{Duration, Utc};

fn main() {
    let end_time = Utc::now();
    let start_time = end_time - Duration::days(30);
    let start_timestamp = start_time.timestamp_millis() as u64;
    let end_timestamp = end_time.timestamp_millis() as u64;
    
    println!("Current time: {}", end_time);
    println!("Start time: {}", start_time);
    println!("Start timestamp: {}", start_timestamp);
    println!("End timestamp: {}", end_timestamp);
    println!("Difference: {} ms", end_timestamp - start_timestamp);
}
