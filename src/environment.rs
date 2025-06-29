use std::env;

pub struct Config {
    pub file_provider: String,
    pub aws_secret_key: String,
    pub aws_region: String,
    pub aws_service: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let file_provider = env::var("FILE_PROVIDER")
            .map_err(|_| "FILE_PROVIDER environment variable not set".to_string())?;
        let aws_secret_key = env::var("AWS_SECRET_KEY")
            .map_err(|_| "AWS_SECRET_KEY environment variable not set".to_string())?;
        let aws_region = env::var("AWS_REGION")
            .map_err(|_| "FILE_PROVIDER environment variable not set".to_string())?;
        let aws_service = env::var("AWS_REGION_SERVICE")
            .map_err(|_| "FILE_PROVIDER environment variable not set".to_string())?;
        Ok(Config {
            file_provider,
            aws_secret_key,
            aws_region,
            aws_service,
        })
    }
}