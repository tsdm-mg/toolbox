use crate::cmd::ProfileArgs;
use anyhow::Result;
use tm_api::profile::{fetch_user_profile_by_id, fetch_user_profile_by_name};

pub async fn run_profile_command(args: ProfileArgs) -> Result<()> {
    let profile = match (args.profile_target.name, args.profile_target.uid) {
        (Some(name), None) => fetch_user_profile_by_name(name).await,
        (None, Some(uid)) => fetch_user_profile_by_id(uid).await,
        _ => unreachable!(),
    };
    println!("{profile:#?}");

    Ok(())
}
