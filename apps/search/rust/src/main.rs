mod bridge;
mod config;
mod database;
mod models;
mod repository;
mod search;
mod server;
mod text;

//async setup so the server can handle multiple requests concurrently
//validates the environment variables like
//establishes a connection pool to the database
//claims a tcp port on the host machine

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::from_env()?;
    let db = database::connect(&config.database_url).await?;
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;

    eprintln!("Search listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        server::router(db.clone(), config.candidate_limit, config.service_token),
    )
    .await?;
    db.close().await;
    Ok(())
}
