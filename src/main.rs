// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use gethostname::gethostname;
use tower_http::services::ServeDir;

pub use self::error::{Error, Result};

pub mod api;
pub mod cleanup;
pub mod error;
pub mod jobs;
pub mod models;
pub mod query;
pub mod search;

use jobs::comparippson::COMPARIPPSON_METADATA;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Base directory for job files
    #[arg(long, short)]
    jobdir: Option<PathBuf>,

    /// Directory containing the antiSMASH outputs
    #[arg(long, short)]
    outdir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Serve the web API
    Serve {
        /// Address to listen on
        #[arg(long, short, default_value = "[::]:5566")]
        address: String,
    },
    /// Run the background jobs
    Run {
        /// Name to register the job runner as
        #[arg(long, short)]
        name: Option<String>,

        /// Base directory for the databases
        #[arg(long, short = 'D')]
        dbdir: Option<PathBuf>,

        /// Base directory for stored job URLs
        #[arg(long, short)]
        urlroot: Option<String>,
    },
    /// Clean up old jobs from the database and file system
    Cleanup {
        /// Days after which to cleanup jobs
        #[arg(long, short, default_value_t = 7.0_f64)]
        interval: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    match dotenv() {
        Ok(_) => eprintln!("->> Loaded variables from .env file"),
        Err(_) => eprintln!("->> Failed to load .env file"),
    }

    let cli = Cli::parse();

    let jobdir = if let Some(d) = cli.jobdir {
        d
    } else {
        if let Ok(d) = env::var("JOBDIR") {
            PathBuf::from(d)
        } else {
            let mut d = env::current_dir()?;
            d.push("jobs");
            d
        }
    };

    let outdir = cli.outdir;

    // TODO: Maybe also add a CLI arg?
    let url = env::var("DATABASE_URL")?;
    let pool = sqlx::postgres::PgPool::connect(&url).await?;

    match &cli.command {
        Commands::Serve { address } => {
            let mut routes_all = api::init_routes(pool);

            if let Some(o) = outdir {
                let serve_dir = ServeDir::new(&o);
                routes_all = routes_all.nest_service("/output", serve_dir);
                eprintln!("->> Serving files from {o:?}");
            }

            let addr: SocketAddr = address.as_str().parse().unwrap();
            eprintln!("->> Listening on {addr}");

            axum::Server::bind(&addr)
                .serve(routes_all.into_make_service())
                .await
                .unwrap();
        }
        Commands::Run {
            name,
            dbdir,
            urlroot,
        } => {
            let config = create_config(name, dbdir, &jobdir, &outdir, &urlroot).await?;
            eprintln!("->> Running the background jobs as {}", config.name);
            jobs::dispatch(pool, config).await.unwrap();
        }
        Commands::Cleanup { interval } => {
            let days = interval.to_owned();
            if days < 0.0 {
                eprintln!("Can't use a negative interval");
                return Err(Error::InvalidRequest(
                    "Can't use a negative interval".to_string(),
                ));
            }

            eprintln!("->> Cleaning up outdated/deleted jobs older than {days} days");
            cleanup::run(&pool, &jobdir, days).await.unwrap();
        }
    }

    Ok(())
}

async fn create_config(
    name: &Option<String>,
    dbdir: &Option<PathBuf>,
    jobdir: &PathBuf,
    outdir: &Option<PathBuf>,
    urlroot: &Option<String>,
) -> Result<jobs::RunConfig> {
    let name_to_use = if let Some(n) = name {
        n.to_owned()
    } else {
        match gethostname().into_string() {
            Ok(hostname) => hostname,
            Err(original) => return Err(Error::OsStringError(original)),
        }
    };

    let db_base_dir = if let Some(d) = dbdir {
        d.to_owned()
    } else {
        if let Ok(d) = env::var("DBDIR") {
            PathBuf::from(d)
        } else {
            let mut d = env::current_dir()?;
            d.push("databases");
            d
        }
    };

    let mut metadata_file = db_base_dir.clone();
    metadata_file.push(COMPARIPPSON_METADATA);

    let metadata =
        jobs::comparippson::Metadata::from_json(&tokio::fs::read_to_string(&metadata_file).await?)?;

    let comparippson_config = jobs::comparippson::CompaRiPPsonConfig {
        metadata,
        dbdir: db_base_dir.clone(),
    };

    let job_dl_url_root = if let Some(u) = urlroot {
        u.to_owned()
    } else {
        "job_downloads".to_string()
    };

    let config = jobs::RunConfig {
        comparippson_config,
        name: name_to_use,
        dbdir: db_base_dir,
        jobdir: jobdir.clone(),
        outdir: outdir.clone(),
        urlroot: job_dl_url_root,
    };
    Ok(config)
}
