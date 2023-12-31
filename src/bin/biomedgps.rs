#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use biomedgps::api::route::BiomedgpsApi;
use biomedgps::init_logger;
use dotenv::dotenv;
use log::LevelFilter;
use poem::middleware::AddData;
use poem::EndpointExt;
use poem::{
    async_trait,
    endpoint::EmbeddedFilesEndpoint,
    handler,
    http::{header, Method, StatusCode},
    listener::TcpListener,
    middleware::Cors,
    web::Redirect,
    Endpoint, Request, Response, Result, Route, Server,
};
use poem_openapi::OpenApiService;
use rust_embed::RustEmbed;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
// use tokio::{self, time::Duration};

use structopt::StructOpt;

/// BioMedGPS backend server.
#[derive(Debug, PartialEq, StructOpt)]
#[structopt(setting=structopt::clap::AppSettings::ColoredHelp, name="biomedgps", author="Jingcheng Yang <yjcyxky@163.com>")]
struct Opt {
    /// Activate debug mode
    /// short and long flags (--debug) will be deduced from the field's name
    #[structopt(name = "debug", long = "debug")]
    debug: bool,

    /// Activate ui mode
    #[structopt(name = "ui", short = "u", long = "ui")]
    ui: bool,

    /// Activate openapi mode
    #[structopt(name = "openapi", short = "o", long = "openapi")]
    openapi: bool,

    /// 127.0.0.1 or 0.0.0.0
    #[structopt(name = "host", short = "H", long = "host", possible_values=&["127.0.0.1", "0.0.0.0"], default_value = "127.0.0.1")]
    host: String,

    /// Which port.
    #[structopt(name = "port", short = "p", long = "port", default_value = "3000")]
    port: String,

    /// Database url, such as postgres:://user:pass@host:port/dbname.
    /// You can also set it with env var: DATABASE_URL.
    #[structopt(name = "database-url", short = "d", long = "database-url")]
    database_url: Option<String>,

    /// Graph Database url, such as neo4j:://user:pass@host:port/dbname.
    /// You can also set it with env var: NEO4J_URL.
    #[structopt(name = "neo4j-url", short = "g", long = "neo4j-url")]
    neo4j_url: Option<String>,

    /// JWT secret key.
    /// You can also set it with env var: JWT_SECRET_KEY.
    /// If you don't set it, the server will disable JWT verification. You can use the API with Authorization header and set it to any value.
    #[structopt(name = "jwt-secret-key", short = "k", long = "jwt-secret-key")]
    jwt_secret_key: Option<String>,
}

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

pub(crate) struct HtmlEmbed;

#[async_trait]
impl Endpoint for HtmlEmbed {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if req.method() != Method::GET {
            return Ok(StatusCode::METHOD_NOT_ALLOWED.into());
        }

        match Assets::get("index.html") {
            Some(content) => {
                let body: Vec<u8> = content.data.into();
                warn!("If you found 404 error when getting static files, please check your frontend's configuration. You might need to set the publicPath to `/assets/`.");
                Ok(Response::builder()
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(body))
            }
            None => Ok(Response::builder().status(StatusCode::NOT_FOUND).finish()),
        }
    }
}

#[handler]
async fn index() -> Redirect {
    Redirect::moved_permanent("/index.html")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    dotenv().ok();

    let args = Opt::from_args();

    let log_result = if args.debug {
        init_logger("biomedgps", LevelFilter::Debug)
    } else {
        init_logger("biomedgps", LevelFilter::Info)
    };

    if let Err(log) = log_result {
        error!(target:"stdout", "Log initialization error, {}", log);
        std::process::exit(1);
    };

    let host = args.host;
    let port = args.port;

    println!("\n\t\t*** Launch biomedgps on {}:{} ***", host, port);

    let database_url = args.database_url;

    let database_url = if database_url.is_none() {
        match std::env::var("DATABASE_URL") {
            Ok(v) => v,
            Err(_) => {
                error!("{}", "DATABASE_URL is not set.");
                std::process::exit(1);
            }
        }
    } else {
        database_url.unwrap()
    };

    if args.jwt_secret_key.is_none() {
        match std::env::var("JWT_SECRET_KEY") {
            Ok(v) => {
                if v.is_empty() {
                    warn!("You don't set JWT_SECRET_KEY environment variable, so we will skip JWT verification, but users also need to set the Authorization header to access the API.");
                    None
                } else {
                    Some(v)
                }
            }
            Err(_) => {
                warn!("You don't set JWT_SECRET_KEY environment variable, so we will skip JWT verification, but users also need to set the Authorization header to access the API.");
                None
            }
        }
    } else {
        std::env::set_var("JWT_SECRET_KEY", args.jwt_secret_key.unwrap());
        None
    };

    // let neo4j_url = args.neo4j_url;

    // let _neo4j_url = if neo4j_url.is_none() {
    //     match std::env::var("NEO4J_URL") {
    //         Ok(v) => v,
    //         Err(_) => {
    //             error!("{}", "NEO4J_URL is not set.");
    //             std::process::exit(1);
    //         }
    //     }
    // } else {
    //     neo4j_url.unwrap()
    // };

    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    let arc_pool = Arc::new(pool);
    let shared_rb = AddData::new(arc_pool.clone());

    let api_service = OpenApiService::new(BiomedgpsApi, "BioMedGPS", "v0.1.0")
        .summary("A RESTful API Service for BioMedGPS.")
        .description("A knowledge graph system with graph neural network for drug discovery, disease mechanism and biomarker screening.")
        .license("GNU AFFERO GENERAL PUBLIC LICENSE v3")
        .server(format!("http://{}:{}", host, port));
    let openapi = api_service.swagger_ui();
    let mut spec = api_service.spec();

    // Remove charset=utf-8 from spec for compatibility with Apifox.
    spec = spec.replace("; charset=utf-8", "");

    let route = Route::new();

    let route = if args.openapi {
        info!("OpenApi mode is enabled. You can access the OpenApi spec at /openapi.");
        route
            .nest("/openapi", openapi)
            .at("/spec", poem::endpoint::make_sync(move |_| spec.clone()))
    } else {
        warn!("OpenApi mode is disabled. If you need the OpenApi, please use `--openapi` flag.");
        route
    };

    let route = if args.ui {
        info!("UI mode is enabled.");
        route
            .at("/", HtmlEmbed)
            .nest("/index.html", HtmlEmbed)
            .nest("/assets", EmbeddedFilesEndpoint::<Assets>::new())
    } else {
        warn!("UI mode is disabled. If you need the UI, please use `--ui` flag.");
        route
    };

    let route = route.nest_no_strip("/api/v1", api_service);

    let route = route.with(Cors::new()).with(shared_rb);

    Server::new(TcpListener::bind(format!("{}:{}", host, port)))
        .run(route)
        .await
    // Server::new(TcpListener::bind(format!("{}:{}", host, port)))
    //   .run_with_graceful_shutdown(
    //     route,
    //     async move {
    //       let _ = tokio::signal::ctrl_c().await;
    //     },
    //     Some(Duration::from_secs(5)),
    //   )
    //   .await
}
