use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub values: HashMap<String, String>,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }
}

/// Process the given config and return a formatted string.
pub fn process(config: &Config) -> String {
    format!("Processing: {}", config.name)
}

const MAX_RETRIES: u32 = 3;
