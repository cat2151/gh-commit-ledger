use crate::cache::CacheStore;
use crate::report::{ProgressUpdate, ReportOutcome};

pub struct LoadedReport {
    pub outcome: ReportOutcome,
    pub cache: CacheStore,
}

pub enum AppMessage {
    Progress(ProgressUpdate),
    Finished(Result<LoadedReport, String>),
}
