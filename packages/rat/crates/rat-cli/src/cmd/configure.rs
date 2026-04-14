use std::collections::HashMap;

use anyhow::Result;
use clap::Subcommand;
use dialoguer::console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;

use rat_cli::config::{self, Profile, RatConfig};

#[derive(Subcommand)]
pub enum ConfigureAction {
    /// List all configured profiles
    List,
    /// Show current profile settings
    Show,
}

pub fn handle(action: Option<ConfigureAction>, profile_name: Option<&str>) -> Result<()> {
    match action {
        Some(ConfigureAction::List) => list(),
        Some(ConfigureAction::Show) => show(profile_name),
        None => interactive(profile_name),
    }
}

fn list() -> Result<()> {
    let cfg = config::load_config()?;
    match cfg {
        None => {
            eprintln!("No configuration found. Run `rat configure` first.");
        }
        Some(cfg) => {
            println!("default");
            for name in cfg.profiles.keys() {
                println!("{}", name);
            }
        }
    }
    Ok(())
}

fn show(profile_name: Option<&str>) -> Result<()> {
    let cfg = config::load_config()?;
    let Some(cfg) = cfg else {
        eprintln!("No configuration found. Run `rat configure` first.");
        return Ok(());
    };

    let label = profile_name.unwrap_or("default");
    let Some(profile) = config::resolve_profile(&cfg, profile_name) else {
        eprintln!("Profile '{}' not found.", label);
        return Ok(());
    };

    println!("[{}]", label);
    println!("aws_region               = {}", profile.aws_region);
    println!("cognito_domain           = {}", profile.cognito_domain);
    println!("cognito_app_client_id    = {}", profile.cognito_app_client_id);
    println!("cognito_identity_pool_id = {}", profile.cognito_identity_pool_id);
    println!("cognito_user_pool_id     = {}", profile.cognito_user_pool_id);
    if !profile.sqs_queue_url.is_empty() {
        println!("sqs_queue_url            = {}", profile.sqs_queue_url);
    }
    if !profile.api_function_arn.is_empty() {
        println!("api_function_arn         = {}", profile.api_function_arn);
    }
    Ok(())
}

fn interactive(profile_name: Option<&str>) -> Result<()> {
    let label = profile_name.unwrap_or("default");
    eprintln!("Configuring profile: {}", label);

    let existing = config::load_config()?.and_then(|c| config::resolve_profile(&c, profile_name));

    let theme = ColorfulTheme {
        active_item_style: Style::new().color256(183),
        ..ColorfulTheme::default()
    };

    let aws_region: String = Input::with_theme(&theme)
        .with_prompt("AWS Region")
        .with_initial_text(existing.as_ref().map_or("ap-northeast-2", |p| &p.aws_region))
        .interact_text()?;

    let cognito_domain: String = Input::with_theme(&theme)
        .with_prompt("Cognito Domain")
        .with_initial_text(existing.as_ref().map_or("", |p| &p.cognito_domain))
        .interact_text()?;

    let cognito_app_client_id: String = Input::with_theme(&theme)
        .with_prompt("Cognito App Client ID")
        .with_initial_text(existing.as_ref().map_or("", |p| &p.cognito_app_client_id))
        .interact_text()?;

    let cognito_identity_pool_id: String = Input::with_theme(&theme)
        .with_prompt("Cognito Identity Pool ID")
        .with_initial_text(existing.as_ref().map_or("", |p| &p.cognito_identity_pool_id))
        .interact_text()?;

    let cognito_user_pool_id: String = Input::with_theme(&theme)
        .with_prompt("Cognito User Pool ID")
        .with_initial_text(existing.as_ref().map_or("", |p| &p.cognito_user_pool_id))
        .interact_text()?;

    let new_profile = Profile {
        aws_region,
        cognito_domain,
        cognito_app_client_id,
        cognito_identity_pool_id,
        cognito_user_pool_id,
        sqs_queue_url: existing.as_ref().map_or(String::new(), |p| p.sqs_queue_url.clone()),
        api_function_arn: existing.as_ref().map_or(String::new(), |p| p.api_function_arn.clone()),
        migration_function_arn: existing.as_ref().map_or(String::new(), |p| p.migration_function_arn.clone()),
    };

    let mut cfg = config::load_config()?.unwrap_or_else(|| RatConfig {
        default: new_profile.clone(),
        profiles: HashMap::new(),
    });

    match profile_name {
        None | Some("default") => {
            cfg.default = new_profile;
        }
        Some(name) => {
            cfg.profiles.insert(name.to_string(), new_profile);
        }
    }

    config::save_config(&cfg)?;
    eprintln!("Configuration saved to {}", config::config_path()?.display());
    Ok(())
}
