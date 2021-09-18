use crate::{
    error::Error,
    jobs::{instance::QueryInstance, nodeinfo::QueryNodeinfo, JobState},
};
use background_jobs::ActixJob;
use std::{future::Future, pin::Pin};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct Listeners;

impl Listeners {
    #[tracing::instrument(name = "Spawn query instances")]
    async fn perform(self, state: JobState) -> Result<(), Error> {
        for actor_id in state.state.db.connected_ids().await? {
            state
                .job_server
                .queue(QueryInstance::new(actor_id.clone()))?;
            state.job_server.queue(QueryNodeinfo::new(actor_id))?;
        }

        Ok(())
    }
}

impl ActixJob for Listeners {
    type State = JobState;
    type Future = Pin<Box<dyn Future<Output = Result<(), anyhow::Error>>>>;

    const NAME: &'static str = "relay::jobs::Listeners";

    fn run(self, state: Self::State) -> Self::Future {
        Box::pin(async move { self.perform(state).await.map_err(Into::into) })
    }
}
