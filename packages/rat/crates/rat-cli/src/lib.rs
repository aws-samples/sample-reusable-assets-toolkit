pub mod auth;
pub mod aws;
pub mod chunk;
pub mod cmd;
pub mod config;
pub mod git;
pub mod highlight;
pub mod ratignore;
pub mod session;
pub mod sqs;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum SearchScope {
    Code,
    Doc,
    Repo,
}
