extern crate log;

use biomedgps::{import_data, import_graph_data, init_logger, run_migrations};
use log::*;
use structopt::StructOpt;

/// A cli for rnmpdb.
#[derive(StructOpt, Debug)]
#[structopt(setting=structopt::clap::AppSettings::ColoredHelp, name = "A cli for biomedgps service.", author="Jingcheng Yang <yjcyxky@163.com>;")]
struct Opt {
    /// Activate debug mode
    /// short and long flags (--debug) will be deduced from the field's name
    #[structopt(name = "debug", long = "debug")]
    debug: bool,

    #[structopt(subcommand)]
    cmd: SubCommands,
}

#[derive(Debug, PartialEq, StructOpt)]
enum SubCommands {
    #[structopt(name = "initdb")]
    InitDB(InitDbArguments),
    #[structopt(name = "importdb")]
    ImportDB(ImportDBArguments),
    #[structopt(name = "importgraph")]
    ImportGraph(ImportGraphArguments),
}

/// Init database.
#[derive(StructOpt, PartialEq, Debug)]
#[structopt(setting=structopt::clap::AppSettings::ColoredHelp, name="BioMedGPS - initdb", author="Jingcheng Yang <yjcyxky@163.com>")]
pub struct InitDbArguments {
    /// Database url, such as postgres://postgres:postgres@localhost:5432/rnmpdb, if not set, use the value of environment variable DATABASE_URL.
    #[structopt(name = "database_url", short = "d", long = "database-url")]
    database_url: Option<String>,
}

/// Import data files into database.
#[derive(StructOpt, PartialEq, Debug)]
#[structopt(setting=structopt::clap::AppSettings::ColoredHelp, name="BioMedGPS - importdb", author="Jingcheng Yang <yjcyxky@163.com>")]
pub struct ImportDBArguments {
    /// Database url, such as postgres://postgres:postgres@localhost:5432/rnmpdb, if not set, use the value of environment variable DATABASE_URL.
    #[structopt(name = "database_url", short = "d", long = "database-url")]
    database_url: Option<String>,

    /// The file path of the data file to import. It may be a file or a directory.
    #[structopt(name = "filepath", short = "f", long = "filepath")]
    filepath: Option<String>,

    /// The table name to import data into. supports entity, entity2d, relation, relation_metadata, entity_metadata, knowledge_curation, subgraph, entity_embedding, relation_embedding
    #[structopt(name = "table", short = "t", long = "table")]
    table: String,

    /// Drop the table before import data. If you have multiple files to import, don't use this option. If you use this option, only the last file will be imported successfully.
    #[structopt(name = "drop", short = "D", long = "drop")]
    drop: bool,

    /// Don't check other related tables in the database. Such as knowledge_curation which might be related to entity.
    #[structopt(name = "skip_check", short = "s", long = "skip-check")]
    skip_check: bool,

    /// Show the first 3 errors when import data.
    #[structopt(name = "show_all_errors", short = "e", long = "show-all-errors")]
    show_all_errors: bool,
}

/// Import data files into a graph database.
#[derive(StructOpt, PartialEq, Debug)]
#[structopt(setting=structopt::clap::AppSettings::ColoredHelp, name="BioMedGPS - importgraph", author="Jingcheng Yang <yjcyxky@163.com>")]
pub struct ImportGraphArguments {
    /// Database url, such as neo4j://<username>:<password>@localhost:7687, if not set, use the value of environment variable NEO4J_URL.
    #[structopt(name = "neo4j_url", short = "n", long = "neo4j_url")]
    neo4j_url: Option<String>,

    /// The file path of the data file to import. It may be a file or a directory.
    #[structopt(name = "filepath", short = "f", long = "filepath")]
    filepath: Option<String>,

    /// The file type of the data file to import. It may be entity, relation, entity_attribute and relation_attribute.
    #[structopt(name = "filetype", short = "t", long = "filetype")]
    filetype: Option<String>,

    /// Batch size for import data. Default is 1000.
    #[structopt(name = "batch_size", short = "b", long = "batch-size")]
    batch_size: Option<usize>,

    /// Don't check other related tables in the database. Such as knowledge_curation which might be related to entity.
    #[structopt(name = "skip_check", short = "s", long = "skip-check")]
    skip_check: bool,

    /// Show the first 3 errors when import data.
    #[structopt(name = "show_all_errors", short = "e", long = "show-all-errors")]
    show_all_errors: bool,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let log_result = if opt.debug {
        init_logger("biomedgps-cli", LevelFilter::Debug)
    } else {
        init_logger("biomedgps-cli", LevelFilter::Info)
    };

    match opt.cmd {
        SubCommands::InitDB(arguments) => {
            let database_url = arguments.database_url;

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

            match run_migrations(&database_url).await {
                Ok(_) => info!("Init database successfully."),
                Err(e) => error!("Init database failed: {}", e),
            }
        }
        SubCommands::ImportDB(arguments) => {
            let database_url = if arguments.database_url.is_none() {
                match std::env::var("DATABASE_URL") {
                    Ok(v) => v,
                    Err(_) => {
                        error!("{}", "DATABASE_URL is not set.");
                        std::process::exit(1);
                    }
                }
            } else {
                arguments.database_url.unwrap()
            };

            if arguments.table.is_empty() {
                error!("Please specify the table name.");
                return;
            };

            import_data(
                &database_url,
                &arguments.filepath,
                &arguments.table,
                arguments.drop,
                arguments.skip_check,
                arguments.show_all_errors,
            )
            .await
        }
        SubCommands::ImportGraph(arguments) => {
            let neo4j_url = if arguments.neo4j_url.is_none() {
                match std::env::var("NEO4J_URL") {
                    Ok(v) => v,
                    Err(_) => {
                        error!("{}", "NEO4J_URL is not set.");
                        std::process::exit(1);
                    }
                }
            } else {
                arguments.neo4j_url.unwrap()
            };

            // Get host, username and password from neo4j_url.
            let mut host = "";
            let mut username = "";
            let mut password = "";
            if neo4j_url.starts_with("neo4j://") {
                let mut parts = neo4j_url.split("neo4j://");
                host = parts.next().unwrap();
                let mut parts = parts.next().unwrap().split(":");
                username = parts.next().unwrap();
                password = parts.next().unwrap();
            } else {
                error!("Invalid neo4j_url: {}", neo4j_url);
                std::process::exit(1);
            }

            let filetype = if arguments.filetype.is_none() {
                error!("Please specify the file type.");
                std::process::exit(1);
            } else {
                arguments.filetype.unwrap()
            };

            let batch_size = if arguments.batch_size.is_none() {
                1000
            } else {
                arguments.batch_size.unwrap()
            };

            import_graph_data(
                host,
                username,
                password,
                &arguments.filepath,
                &filetype,
                arguments.skip_check,
                arguments.show_all_errors,
                batch_size,
            )
            .await
        }
    }
}
