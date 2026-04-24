use crate::cache::CacheStore;
use crate::events::{AppMessage, LoadedReport};
use crate::gh::GhClient;
use crate::report::generate_report;
use tokio::sync::mpsc::UnboundedSender;

pub(crate) fn spawn_report_task(
    tx: UnboundedSender<AppMessage>,
    client: GhClient,
    cache_path: String,
    force_refresh: bool,
) {
    tokio::spawn(async move {
        let load_result = async {
            let mut cache = CacheStore::load_from_path(cache_path)?;
            let outcome = generate_report(&client, &mut cache, force_refresh, &tx).await?;
            Ok::<LoadedReport, anyhow::Error>(LoadedReport { outcome, cache })
        }
        .await
        .map_err(|error| error.to_string());

        let _ = tx.send(AppMessage::Finished(load_result));
    });
}
