use std::{env, io, net::SocketAddr};

pub struct Config {
    pub service_token: String,
    pub database_url: String,
    pub listen_addr: SocketAddr,
    pub candidate_limit: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let service_token = env::var("SEARCH_SERVICE_TOKEN")?;
        if service_token.is_empty() || !service_token.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SEARCH_SERVICE_TOKEN must be nonempty printable ASCII without spaces",
            )
            .into());
        }
        let database_url = env::var("DATABASE_URL")?;
        if database_url.trim().is_empty() {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "DATABASE_URL is empty").into(),
            );
        }
        // use 0.0.0.0:8080 inside Docker, explicitly.
        let listen_addr = env::var("SEARCH_LISTEN_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8081".to_owned())
            .parse()?;
        let candidate_limit = env::var("SEARCH_CANDIDATE_LIMIT")
            .unwrap_or_else(|_| "2000".to_owned())
            .parse::<u64>()?;
        if !(1..=10_000).contains(&candidate_limit) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SEARCH_CANDIDATE_LIMIT must be between 1 and 10,000",
            )
            .into());
        }
        Ok(Self {
            service_token,
            database_url,
            listen_addr,
            candidate_limit,
        })
    }
}
