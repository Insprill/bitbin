#![forbid(unsafe_code)]

use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
    process::exit,
    time::Duration,
};

use actix_web::{middleware, web::Data, App, HttpServer};
use anyhow::Result;
use config::{BitbinConfig, ServerConfig};
use log::{error, info, warn, LevelFilter};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rustls::{Certificate, PrivateKey, ServerConfig as RustlsServerConfig};

use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode};

use crate::config::Config;

mod config;
mod db;
mod get;
mod post;

pub struct State {
    pool: Pool<SqliteConnectionManager>,
    config: BitbinConfig,
}

#[actix_web::main]
async fn main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        simplelog::Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    let root_config = Config::create()?;
    let config = root_config.server;

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| config.port.to_string())
        .parse::<u16>()
        .unwrap();

    info!(
        "Starting bitbin v{}, listening on {}:{}!",
        env!("CARGO_PKG_VERSION"),
        config.host,
        port
    );

    let db_dir = PathBuf::from("db");
    if !db_dir.exists() {
        let _ = fs::create_dir(db_dir);
    }
    let manager = SqliteConnectionManager::file("db/bitbin.db");
    let pool = Pool::new(manager).unwrap();

    let _ = db::create_db(pool.get()?);

    let data = Data::new(State {
        pool,
        config: root_config.bitbin,
    });

    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(middleware::Compress::default())
            // Routes
            .service(post::post)
            .service(get::get)
    });

    if config.keep_alive_timeout > 0.0 {
        server = server.keep_alive(Duration::from_secs_f32(config.keep_alive_timeout));
    } else {
        server = server.keep_alive(None)
    }

    if config.workers > 0 {
        server = server.workers(config.workers);
    }

    let _ = if config.tls {
        // To create a self-signed temporary cert for testing:
        // openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'
        server.bind_rustls_021((config.host.to_owned(), port), build_tls_config(&config)?)
    } else {
        server.bind_auto_h2c((config.host, port))
    }?
    .run()
    .await;

    Ok(())
}

fn build_tls_config(config: &ServerConfig) -> std::io::Result<RustlsServerConfig> {
    Ok(RustlsServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(create_cert_chain(config), PrivateKey(create_key(config)))
        .unwrap())
}

fn create_cert_chain(config: &ServerConfig) -> Vec<Certificate> {
    let cert_file_path = config.tls_cert_file.as_ref().unwrap();
    let cert_file = &mut BufReader::new(match File::open(cert_file_path) {
        Ok(file) => file,
        Err(err) => {
            error!("Failed to load cert file '{}': {}", cert_file_path, err);
            exit(1);
        }
    });

    let cert_chain: Vec<Certificate> = rustls_pemfile::certs(cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    if cert_chain.is_empty() {
        error!("Failed to find any certs in '{}'", cert_file_path);
        exit(1);
    }
    cert_chain
}

fn create_key(config: &ServerConfig) -> Vec<u8> {
    let key_file_path = config.tls_key_file.as_ref().unwrap();
    let key_file = &mut BufReader::new(match File::open(key_file_path) {
        Ok(file) => file,
        Err(err) => {
            error!("Failed to load key file '{}': {}", key_file_path, err);
            exit(1);
        }
    });
    let mut keys: Vec<Vec<u8>> = rustls_pemfile::pkcs8_private_keys(key_file).unwrap();
    if keys.is_empty() {
        error!("Failed to find any keys in '{}'", key_file_path);
        exit(1);
    }
    if keys.len() > 1 {
        warn!(
            "Found multiple keys in '{}'! Only the first will be used.",
            key_file_path
        );
    }

    keys.remove(0)
}
