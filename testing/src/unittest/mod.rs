pub mod setup {
    pub mod log {
        use std::sync::Once;
        static SETUP: Once = Once::new();
        pub fn configure() {
            configure_level(log::LevelFilter::Trace)
        }
        pub fn configure_level(level: log::LevelFilter) {
            SETUP.call_once(|| {
                let _ = env_logger::builder()
                    .format_timestamp_micros()
                    // .is_test(true) // disables color in the terminal
                    .filter_level(level)
                    .try_init();
            });
        }
    }
    pub mod net {
        use std::{net::TcpListener, ops::Range, time::Duration};

        pub fn find_available_port(range: Range<u16>) -> u16 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            for _ in 0..1000 {
                let port = rng.gen_range(range.clone());
                if TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
                    return port;
                }
            }
            panic!("Unable to find an available port in range: {:?}", range);
        }

        pub fn rand_avail_addr_port() -> &'static str {
            let port = find_available_port(8000..9000);
            let addr = format!("0.0.0.0:{}", port).into_boxed_str();
            Box::leak(addr)
        }

        pub fn default_connect_timeout() -> Duration {
            Duration::from_millis(500)
        }
        pub fn default_connect_retry_after() -> Duration {
            default_connect_timeout() / 5
        }

        pub fn optional_find_timeout() -> Option<Duration> {
            Some(Duration::from_millis(1))
        }
    }
}
