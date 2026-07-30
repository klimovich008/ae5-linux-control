use std::error::Error;
use std::thread;

fn main() {
    if let Err(error) = run() {
        eprintln!("ae5d: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let _connection = ae5_control::device_service::serve()?;
    eprintln!(
        "ae5d event=service-ready bus=session writes=volume,mute,headphone-gain,sample-rate-policy,profile-library,software-eq,hardware-effects,software-effects-fallback"
    );
    loop {
        thread::park();
    }
}
