#[macro_export]
macro_rules! clog {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[macro_export]
macro_rules! log_to_console {
    ( $( $t:tt )* ) => {
        #[cfg(target_arch = "wasm32")]
        $crate::clog!( $( $t )* );
        #[cfg(not(target_arch = "wasm32"))]
        println!( $( $t )* );
    }
}

#[macro_export]
macro_rules! retry {
    ($expr:expr, $sleep:expr, $message:expr) => {
        retry!($expr, $sleep, $message, 0)
    };
    ($expr:expr, $sleep:expr, $message:expr, $limit:expr) => {{
        let mut retries = 0;
        loop {
            match $expr {
                Ok(val) => break Ok(val),
                Err(e) => {
                    retries += 1;
                    warn!("{}", e);
                    if $limit > 0 && retries >= $limit {
                        error!("Retry limit reached: {}", $message);
                        break Err(e);
                    }
                    info!(
                        "Waiting {} seconds before retrying {} (retry {}/{})",
                        $sleep, $message, retries, $limit
                    );
                    std::thread::sleep(std::time::Duration::from_secs($sleep));
                }
            }
        }
    }};
}
