use crate::analyze::load_thread_data_from_dir;
use crate::cmd::ProfileArgs;
use crate::utils::ask_delete_if_exists;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tm_api::profile::{fetch_user_profile_by_id, fetch_user_profile_by_name};
use tokio::fs;
use tokio::time::sleep;

pub async fn run_profile_command(args: ProfileArgs) -> Result<()> {
    if let Some(name) = args.profile_target.name {
        let profile = fetch_user_profile_by_name(name).await;
        println!("{profile:#?}");
        return Ok(());
    }

    if let Some(uid) = args.profile_target.uid {
        let profile = fetch_user_profile_by_id(uid).await;
        println!("{profile:#?}");
        return Ok(());
    }

    if let Some(reg_dir) = args.profile_target.thread_data {
        let output = if let Some(v) = args.output {
            v
        } else {
            println!("-o/--output need to be set if you would download all registration profiles");
            return Ok(());
        };

        let output_dir_path = PathBuf::from(output);

        ask_delete_if_exists(&output_dir_path).await?;
        fs::create_dir(&output_dir_path).await?;

        let reg_data = load_thread_data_from_dir(reg_dir.as_str())
            .await
            .with_context(|| format!("when loading registration thread data from dir {reg_dir}"))?;
        for reg in reg_data {
            println!("downloading data for tid={}, page={}", reg.tid, reg.page);
            for post in reg.thread.post_list {
                println!("downloading for floor={} pid={}", post.floor, post.id);
                let uid = post.author_id;

                let profile = fetch_user_profile_by_id(uid.as_str())
                    .await
                    .with_context(|| format!("failed to fetch profile for uid={uid}"))?;

                let mut file_path = output_dir_path.clone();
                file_path.push(format!("post_{}_{}_{}.json", reg.tid, reg.page, post.floor));

                let data = serde_json::to_string_pretty(&profile)
                    .context("failed to serialize user profile")?;
                fs::write(file_path, data).await.with_context(|| {
                    format!(
                        "failed to save user profile tid={}, page={}, floor={}",
                        reg.tid, reg.page, post.floor
                    )
                })?;

                sleep(Duration::from_millis(700)).await;
            }
        }

        return Ok(());
    }

    // let profile = match (args.profile_target.name, args.profile_target.uid) {
    //     (Some(name), None) => unimplemented!(),
    //     (None, Some(uid)) => (uid).await,
    //     _ => unreachable!(),
    // };
    // println!("{profile:#?}");

    Ok(())
}
