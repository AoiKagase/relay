use crate::{error::Error, jobs::JobState};
use background_jobs::{Backoff, Job};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct RecordLastOnline;

impl Job for RecordLastOnline {
    type State = JobState;
    type Error = Error;

    const NAME: &'static str = "relay::jobs::RecordLastOnline";
    const QUEUE: &'static str = "maintenance";
    const BACKOFF: Backoff = Backoff::Linear(1);

    #[tracing::instrument(skip(state))]
    async fn run(self, state: Self::State) -> Result<(), Self::Error> {
        let nodes = state.state.last_online.take();

        state.state.db.mark_last_seen(nodes).await
    }
}
