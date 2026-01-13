use crate::{
    error::Error,
    jobs::{instance::QueryInstance, nodeinfo::QueryNodeinfo, JobState},
};
use background_jobs::Job;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct Listeners;

impl Job for Listeners {
    type State = JobState;
    type Error = Error;

    const NAME: &'static str = "relay::jobs::Listeners";
    const QUEUE: &'static str = "maintenance";

    #[tracing::instrument(name = "Spawn query instances", skip(state))]
    async fn run(self, state: Self::State) -> Result<(), Self::Error> {
        for actor_id in state.state.db.connected_ids().await? {
            state
                .job_server
                .queue(QueryInstance::new(actor_id.clone()))
                .await?;
            state.job_server.queue(QueryNodeinfo::new(actor_id)).await?;
        }

        Ok(())
    }
}
