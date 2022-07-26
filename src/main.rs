use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

use anyhow::Context;
use clap::{Args, Parser};
use reshape::{
    migrations::{Action, Migration},
    Reshape,
};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[clap(name = "Reshape", version, about)]
struct Opts {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Parser)]
#[clap(about)]
enum Command {
    #[clap(subcommand)]
    Migration(MigrationCommand),

    #[clap(
        about = "Output the query your application should use to select the right schema",
        display_order = 2
    )]
    SchemaQuery(FindMigrationsOptions),

    #[clap(
        about = "Deprecated. Use `reshape schema-query` instead",
        display_order = 3
    )]
    GenerateSchemaQuery(FindMigrationsOptions),

    #[clap(
        about = "Deprecated. Use `reshape migration start` instead",
        display_order = 4
    )]
    Migrate(MigrateOptions),
    #[clap(
        about = "Deprecated. Use `reshape migration complete` instead",
        display_order = 5
    )]
    Complete(ConnectionOptions),
    #[clap(
        about = "Deprecated. Use `reshape migration abort` instead",
        display_order = 6
    )]
    Abort(ConnectionOptions),
}

#[derive(Parser)]
#[clap(about = "Commands for managing migrations", display_order = 1)]
enum MigrationCommand {
    #[clap(
        about = "Starts a new migration, applying any migrations which haven't yet been applied",
        display_order = 1
    )]
    Start(MigrateOptions),

    #[clap(about = "Completes an in-progress migration", display_order = 2)]
    Complete(ConnectionOptions),

    #[clap(
        about = "Aborts an in-progress migration without losing any data",
        display_order = 3
    )]
    Abort(ConnectionOptions),
}

#[derive(Args)]
struct MigrateOptions {
    // Some comment
    #[clap(long, short)]
    complete: bool,
    #[clap(flatten)]
    connection_options: ConnectionOptions,
    #[clap(flatten)]
    find_migrations_options: FindMigrationsOptions,
}

#[derive(Parser)]
struct ConnectionOptions {
    #[clap(long)]
    url: Option<String>,
    #[clap(long, default_value = "localhost")]
    host: String,
    #[clap(long, default_value = "5432")]
    port: u16,
    #[clap(long, short, default_value = "postgres")]
    database: String,
    #[clap(long, short, default_value = "postgres")]
    username: String,
    #[clap(long, short, default_value = "postgres")]
    password: String,
}

#[derive(Parser)]
struct FindMigrationsOptions {
    #[clap(long, default_value = "migrations")]
    dirs: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();
    run(opts)
}

fn run(opts: Opts) -> anyhow::Result<()> {
    match opts.cmd {
        Command::Migration(MigrationCommand::Start(opts)) | Command::Migrate(opts) => {
            let mut reshape = reshape_from_connection_options(&opts.connection_options)?;
            let migrations = find_migrations(&opts.find_migrations_options)?;
            reshape.migrate(migrations)?;

            // Automatically complete migration if --complete flag is set
            if opts.complete {
                reshape.complete()?;
            }

            Ok(())
        }
        Command::Migration(MigrationCommand::Complete(opts)) | Command::Complete(opts) => {
            let mut reshape = reshape_from_connection_options(&opts)?;
            reshape.complete()
        }
        Command::Migration(MigrationCommand::Abort(opts)) | Command::Abort(opts) => {
            let mut reshape = reshape_from_connection_options(&opts)?;
            reshape.abort()
        }
        Command::SchemaQuery(opts) | Command::GenerateSchemaQuery(opts) => {
            let migrations = find_migrations(&opts)?;
            let query = migrations
                .last()
                .map(|migration| reshape::schema_query_for_migration(&migration.name));
            println!("{}", query.unwrap_or_else(|| "".to_string()));

            Ok(())
        }
    }
}

fn reshape_from_connection_options(opts: &ConnectionOptions) -> anyhow::Result<Reshape> {
    // Load environment variables from .env file if it exists
    dotenv::dotenv().ok();

    let url_env = std::env::var("DB_URL").ok();
    let url = url_env.as_ref().or_else(|| opts.url.as_ref());

    // Use the connection URL if it has been set
    if let Some(url) = url {
        return Reshape::new(url);
    }

    let host_env = std::env::var("DB_HOST").ok();
    let host = host_env.as_ref().unwrap_or_else(|| &opts.host);

    let port = std::env::var("DB_PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(opts.port);

    let username_env = std::env::var("DB_USERNAME").ok();
    let username = username_env.as_ref().unwrap_or_else(|| &opts.username);

    let password_env = std::env::var("DB_PASSWORD").ok();
    let password = password_env.as_ref().unwrap_or_else(|| &opts.password);

    let database_env = std::env::var("DB_NAME").ok();
    let database = database_env.as_ref().unwrap_or_else(|| &opts.database);

    Reshape::new_with_options(host, port, database, username, password)
}

fn find_migrations(opts: &FindMigrationsOptions) -> anyhow::Result<Vec<Migration>> {
    let search_paths = opts
        .dirs
        .iter()
        .map(Path::new)
        // Filter out all directories that don't exist
        .filter(|path| path.exists());

    // Find all files in the search paths
    let mut file_paths = Vec::new();
    for search_path in search_paths {
        let entries = fs::read_dir(search_path)?;
        for entry in entries {
            let path = entry?.path();
            file_paths.push(path);
        }
    }

    // Sort all files by their file names (without extension)
    // The files are sorted naturally, e.g. "1_test_migration" < "10_test_migration"
    file_paths.sort_unstable_by(|path1, path2| {
        let file1 = path1.as_path().file_stem().unwrap().to_str().unwrap();
        let file2 = path2.as_path().file_stem().unwrap().to_str().unwrap();

        lexical_sort::natural_cmp(file1, file2)
    });

    file_paths
        .iter()
        .map(|path| {
            let mut file = File::open(path)?;

            // Read file data
            let mut data = String::new();
            file.read_to_string(&mut data)?;

            Ok((path, data))
        })
        .map(|result| {
            result.and_then(|(path, data)| {
                let extension = path.extension().and_then(|ext| ext.to_str()).unwrap();
                let file_migration =
                    decode_migration_file(&data, extension).with_context(|| {
                        format!("failed to parse migration file {}", path.display())
                    })?;

                let file_name = path.file_stem().and_then(|name| name.to_str()).unwrap();
                Ok(Migration {
                    name: file_migration.name.unwrap_or_else(|| file_name.to_string()),
                    description: file_migration.description,
                    actions: file_migration.actions,
                })
            })
        })
        .collect()
}

fn decode_migration_file(data: &str, extension: &str) -> anyhow::Result<FileMigration> {
    let migration: FileMigration = match extension {
        "json" => serde_json::from_str(data)?,
        "toml" => toml::from_str(data)?,
        extension => {
            return Err(anyhow::anyhow!(
                "unrecognized file extension '{}'",
                extension
            ))
        }
    };

    Ok(migration)
}

#[derive(Serialize, Deserialize)]
struct FileMigration {
    name: Option<String>,
    description: Option<String>,
    actions: Vec<Box<dyn Action>>,
}
