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
                        let target_map = if thread.revised.as_deref().unwrap_or_default().contains(&post.floor) {
                            println!("group {:?} thread {} floor {}: poll revised as valid", group, thread.name, thread.floor);
                            // Revised as ok.
                            &mut passed_map
                        } else if thread.validate_poll_format(post.body.as_str(), post.floor) {
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
        // Recording continuous ranges.
        let mut ranges: Vec<(&usize, &usize)> = vec![];
        // The start of range that is currently constructing.
        let mut start: Option<&usize> = None;
        // Previous element, use to check whether the current range ended.
        let mut prev: Option<&usize> = None;
        let mut floors = floors.clone();
        floors.sort();

        for f in floors.iter() {
            match start {
                Some(s) => match prev {
                    Some(p) => {
                        if *p == *f - 1 {
                            // Continuous.
                            prev = Some(f);
                        } else {
                            // Broke, generate new range.
                            ranges.push((s, p));
                            start = Some(f);
                            prev = Some(f);
                        }
                    }
                    None => prev = Some(f),
                },
                None => start = Some(f),
            }
        }

        // Clean up the last range.
        if let Some(s) = start {
            ranges.push((s, prev.unwrap_or(s)));
        }

        println!("{thread}:");
        for (start, end) in ranges {
            print!(" ");
            if start == end {
                print!("{start}");
            } else {
                print!("{start}~{end}");
            }
        }
        println!();
    }

    println!("\n");

    println!("invalid polls in each thread: ");
    for (thread, floors) in invalid_map {
        println!("{thread}: {floors:?}");
    }

    Ok(())
}
