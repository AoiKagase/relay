use crate::{
    config::UrlKind,
    db::Actor,
    error::Error,
    jobs::{apub::generate_undo_follow, Deliver, JobState},
};
use background_jobs::Job;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct Reject(pub(crate) Actor);

impl std::fmt::Debug for Reject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reject").field("actor", &self.0.id).finish()
    }
}

impl Job for Reject {
    type State = JobState;
    type Error = Error;

    const NAME: &'static str = "relay::jobs::apub::Reject";
    const QUEUE: &'static str = "apub";

    #[tracing::instrument(name = "Reject", skip(state))]
    async fn run(self, state: Self::State) -> Result<(), Self::Error> {
        state.actors.remove_connection(&self.0).await?;

        let my_id = state.config.generate_url(UrlKind::Actor);
        let undo = generate_undo_follow(&state.config, &self.0.id, &my_id)?;

        state
            .job_server
            .queue(Deliver::new(self.0.inbox, undo)?)
            .await?;

        Ok(())
    }
}
