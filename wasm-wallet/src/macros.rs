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
