pub struct Utils {
    id: u8,
    debug: bool,
    node_name: String,
}

pub enum LogLevel {
    INFO,
    DEBUG,
    ERROR,
}

impl Utils {
    /// Constructor for the Utils struct
    pub fn new(id: u8, debug: bool, node_name: String) -> Utils {
        Utils {
            id,
            debug,
            node_name,
        }
    }

    /// Utility method used to cleanly log information, differentiating on three different levels
    ///
    /// # Args
    /// * `log_message: &str` - the message to log
    /// * `log_level: LogLevel` - the level of the log:
    ///     * `INFO`: default log level, will always be printed
    ///     * `DEBUG`: used only in debug situation, will not print if the debug flag is `false`
    ///     * `ERROR`: will print the message to `io::stderr`
    pub fn log(&self, log_message: &str, log_level: LogLevel) {
        match log_level {
            LogLevel::INFO => {
                print!(
                    "LEVEL: INFO >>> [{} {}] - {}",
                    self.node_name, self.id, log_message
                );
            }
            LogLevel::DEBUG => {
                if self.debug {
                    print!(
                        "LEVEL: DEBUG >>> [{} {}] - {}",
                        self.node_name, self.id, log_message
                    );
                }
            }
            LogLevel::ERROR => {
                eprint!(
                    "LEVEL: ERROR >>> [{} {}] - {}",
                    self.node_name, self.id, log_message
                );
            }
        }
    }
}
