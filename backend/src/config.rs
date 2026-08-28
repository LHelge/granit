use crate::error::ServerError;

/// Server configuration, read from environment variables.
///
/// `DATABASE_URL` is read separately (it is also needed by the CLI
/// subcommands, which don't touch the rest of the configuration).
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub s3: S3Config,
}

#[derive(Debug, Clone)]
pub struct S3Config {
    /// Endpoint the server itself talks to (docker-internal), e.g. `http://rustfs:9000`.
    pub endpoint: String,
    /// Endpoint presigned URLs are signed against — must be the origin the
    /// desktop app can reach (the Caddy origin), e.g. `http://localhost:8080`.
    pub public_endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

fn require(name: &str) -> Result<String, ServerError> {
    std::env::var(name).map_err(|_| ServerError::Config(format!("{name} is not set")))
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ServerError> {
        Ok(Self {
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            s3: S3Config {
                endpoint: require("S3_ENDPOINT")?,
                public_endpoint: require("S3_PUBLIC_ENDPOINT")?,
                bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "granit-backups".to_string()),
                access_key: require("S3_ACCESS_KEY")?,
                secret_key: require("S3_SECRET_KEY")?,
            },
        })
    }
}
