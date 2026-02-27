use std::thread;
use std::time::Duration;
use get_clipboard::util::paste;

fn main() {
    println!("Preparing to simulate system paste (Cmd+V)...");
    println!("You have 3 seconds to switch to a text field.");
    
    thread::sleep(Duration::from_secs(1));
    println!("2...");
    thread::sleep(Duration::from_secs(1));
    println!("1...");
    thread::sleep(Duration::from_secs(1));
    
    let start = std::time::Instant::now();
    match paste::simulate_paste() {
        Ok(_) => println!("Paste simulation command sent in {:?}", start.elapsed()),
        Err(e) => eprintln!("Failed to simulate paste: {}", e),
    }
}
