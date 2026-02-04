fn main() {
    println!("--- Silent-Rails: Connection Initializing ---");

    let user_id = "User_007";
    let security_level = 5;

    println!("[1] Identity check for {}...", user_id);
    
    // Simulating 66ms security latency protocol
    std::thread::sleep(std::time::Duration::from_millis(66));

    if security_level > 3 {
        println!("[2] Access GRANTED via Sentinel Protocol.");
        println!("[3] Encrypted Tunnel established: [STABLE]");
        println!("--- Success: You are now on the Rails ---");
    } else {
        println!("[!] Access DENIED: Insufficient security level.");
    }
}
