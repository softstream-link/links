pub mod setup {
    pub mod log {
        use std::sync::Once;
        static SETUP: Once = Once::new();
        pub fn configure() {
            configure_at(log::LevelFilter::Trace)
        }
        pub fn configure_at(level: log::LevelFilter) {
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
        use std::{net::TcpListener, time::Duration};

        use lazy_static::lazy_static;
        lazy_static! {
            static ref AVAILABLE_PORT: u16 = {
                for port in 8000..9000 {
                    if TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
                        return port;
                    }
                }
                panic!("Unable to find an available port in range 8000..9000");
            };
        }

        pub fn default_addr() -> &'static str {
            let addr = format!("0.0.0.0:{}", *AVAILABLE_PORT).into_boxed_str();
            Box::leak(addr)
        }

        pub fn default_connect_timeout() -> Duration {
            Duration::from_secs_f32(0.5)
        }
        pub fn default_connect_retry_after() -> Duration {
            default_connect_timeout() / 5
        }

        pub fn optional_find_timeout() -> Option<Duration> {
            Some(Duration::from_secs_f32(1.))
        }
    }
}
