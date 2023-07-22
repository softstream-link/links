pub mod setup {
    pub mod log {
        use std::sync::Once;
        static SETUP: Once = Once::new();
        pub fn configure() {
            SETUP.call_once(|| {
                let _ = env_logger::builder()
                    // .is_test(true) // disables color in the terminal
                    .filter_level(log::LevelFilter::Trace)
                    .try_init();
            });
        }
    }
    pub mod net {
        use std::time::Duration;

        pub fn default_addr() -> String {
            String::from("0.0.0.0:8080")
        }
        pub fn default_connect_timeout() -> Duration {
            Duration::from_secs_f32(0.5)
        }
        pub fn default_connect_retry_after() -> Duration {
            default_connect_timeout() / 5
        }

        pub fn default_find_timeout() -> Duration {
            Duration::from_secs_f32(1.)
        }
    }
}
