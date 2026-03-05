use crate::config::Config;
use crate::error::Result;
use crate::operations;
use crate::util;

/// Combined init + sync in one parallel pass.
pub async fn handle_refresh(config: &Config) -> Result<()> {
    let uninitialized = operations::get_uninitialized_repos(config)?;
    let syncable = operations::get_syncable_repos(config)?;

    if uninitialized.is_empty() && syncable.is_empty() {
        println!("Nothing to refresh — vault is empty or all repos are up to date.");
        return Ok(());
    }

    let init_count = uninitialized.len();
    let sync_count = syncable.len();
    println!(
        "Refreshing: {} to init, {} to sync...",
        init_count, sync_count
    );

    // Build combined work list tagged with action type
    #[derive(Clone, Copy)]
    enum Action {
        Init,
        Sync,
    }

    let mut work: Vec<(String, Action)> = Vec::with_capacity(init_count + sync_count);
    for name in uninitialized {
        work.push((name, Action::Init));
    }
    for name in syncable {
        work.push((name, Action::Sync));
    }

    let names: Vec<String> = work.iter().map(|(n, _)| n.clone()).collect();
    let actions: Vec<Action> = work.iter().map(|(_, a)| *a).collect();

    // We need the action type inside the closure. Store it in a shared map.
    let action_map: std::collections::HashMap<String, Action> = names
        .iter()
        .zip(actions.iter())
        .map(|(n, a)| (n.clone(), *a))
        .collect();
    let action_map = std::sync::Arc::new(action_map);

    let config_clone = config.clone();
    let max = config.max_parallel();

    let results = util::run_parallel(names, max, move |name| {
        let action = action_map.get(name).copied().unwrap_or(Action::Sync);
        match action {
            Action::Init => operations::init_pristine(name, None, &config_clone).map(|_| ()),
            Action::Sync => operations::sync_pristine(name, &config_clone),
        }
    })
    .await;

    let mut init_ok = 0usize;
    let mut init_fail = 0usize;
    let mut sync_ok = 0usize;
    let mut sync_fail = 0usize;

    // Rebuild the action lookup for result reporting
    let action_map2: std::collections::HashMap<String, Action> =
        work.iter().map(|(n, a)| (n.clone(), *a)).collect();

    for (name, result) in results {
        let action = action_map2.get(&name).copied().unwrap_or(Action::Sync);
        let ok = matches!(result, Ok(Ok(())));
        match (action, ok) {
            (Action::Init, true) => init_ok += 1,
            (Action::Init, false) => {
                if let Ok(Err(e)) = &result {
                    println!("Failed to init {}: {}", name, e);
                } else if let Err(e) = &result {
                    println!("Task error for {}: {}", name, e);
                }
                init_fail += 1;
            }
            (Action::Sync, true) => sync_ok += 1,
            (Action::Sync, false) => {
                if let Ok(Err(e)) = &result {
                    println!("Failed to sync {}: {}", name, e);
                } else if let Err(e) = &result {
                    println!("Task error for {}: {}", name, e);
                }
                sync_fail += 1;
            }
        }
    }

    println!(
        "\nRefresh complete: init {}/{}, sync {}/{}",
        init_ok,
        init_ok + init_fail,
        sync_ok,
        sync_ok + sync_fail
    );

    Ok(())
}
