use crate::cmd::ValidateArgs;
use crate::config::{Config, ThreadType};
use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::fs;
use tracing::trace;

pub async fn run_validate_command(args: ValidateArgs) -> Result<()> {
    let config_path = args.config;
    let data = fs::read_to_string(config_path)
        .await
        .context("when reading config file")?;
    let config: Config = toml::from_str(data.as_str()).context("invalid config")?;
    trace!("{config:#?}");

    // Load post data, only poll ones.
    let post_data = config
        .load_thread_data(Some(ThreadType::Poll))
        .await
        .context("failed to load thread from config")?;

    // Map holding valid poll result in each thread.
    let mut passed_map = HashMap::<String, Vec<usize>>::new();

    // Map holding invalid poll result in each thread.
    let mut invalid_map = HashMap::<String, Vec<usize>>::new();

    for round in config.round {
        for thread_group in round.group {
            for thread in thread_group.thread {
                let round = &round.name;
                let group = &thread_group.name;
                let name = &thread.name;
                let identifier = format!(
                    "{}{}{}",
                    round,
                    group.as_ref().unwrap_or(&String::new()),
                    name
                );

                for t in post_data.iter().filter(|x| {
                    &x.round == round && x.group.as_ref() == group.as_ref() && &x.name == name
                }) {
                    for post in &t.thread.post_list {
                        let target_map = if thread.validate_poll_format(post.body.as_str()) {
                            // Validate poll result.
                            &mut passed_map
                        } else {
                            // Valid.
                            &mut invalid_map
                        };

                        match target_map.get_mut(&identifier) {
                            Some(v) => {
                                v.push(post.floor);
                            }
                            None => {
                                target_map.insert(identifier.clone(), vec![post.floor]);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("valid polls in each thread: ");
    for (thread, floors) in passed_map {
        println!("{thread}: {floors:?}");
    }

    println!("\n\n\n");

    println!("invalid polls in each thread: ");
    for (thread, floors) in invalid_map {
        println!("{thread}: {floors:?}");
    }

    Ok(())
}
