use std::env;

pub struct Config {
    pub secret_key: String,
    pub file_provider: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let secret_key = env::var("SECRET_KEY")
            .map_err(|_| "SECRET_KEY environment variable not set".to_string())?;
        let file_provider = env::var("FILE_PROVIDER")
            .map_err(|_| "FILE_PROVIDER environment variable not set".to_string())?;
        Ok(Config {
            secret_key,
            file_provider,
        })
    }
}