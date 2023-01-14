macro_rules! retry {
    ($expr:expr, $sleep:expr, $message:expr) => {{
        loop {
            match $expr {
                Ok(val) => break val,
                Err(e) => {
                    warn!("{}", e);
                    info!("Waiting {} seconds before retrying {} ", $sleep, $message);
                    std::thread::sleep(std::time::Duration::from_secs($sleep));
                }
            }
        }
    }};
}
