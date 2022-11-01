use crate::{
    config::UrlKind,
    error::{Error, ErrorKind},
    jobs::{cache_media::CacheMedia, JobState},
};
use activitystreams::{iri, iri_string::types::IriString};
use background_jobs::ActixJob;
use std::{future::Future, pin::Pin};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct QueryInstance {
    actor_id: IriString,
}

impl std::fmt::Debug for QueryInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryInstance")
            .field("actor_id", &self.actor_id.to_string())
            .finish()
    }
}

impl QueryInstance {
    pub(crate) fn new(actor_id: IriString) -> Self {
        QueryInstance { actor_id }
    }

    #[tracing::instrument(name = "Query instance", skip(state))]
    async fn perform(self, state: JobState) -> Result<(), Error> {
        let contact_outdated = state
            .node_cache
            .is_contact_outdated(self.actor_id.clone())
            .await;
        let instance_outdated = state
            .node_cache
            .is_instance_outdated(self.actor_id.clone())
            .await;

        if !(contact_outdated || instance_outdated) {
            return Ok(());
        }

        let authority = self
            .actor_id
            .authority_str()
            .ok_or(ErrorKind::MissingDomain)?;
        let scheme = self.actor_id.scheme_str();
        let instance_uri = iri!(format!("{}://{}/api/v1/instance", scheme, authority));

        let instance = state
            .requests
            .fetch_json::<Instance>(instance_uri.as_str())
            .await?;

        let description = if instance.description.is_empty() {
            instance.short_description.unwrap_or_default()
        } else {
            instance.description
        };

        if let Some(mut contact) = instance.contact {
            let uuid = if let Some(uuid) = state.media.get_uuid(contact.avatar.clone()).await? {
                contact.avatar = state.config.generate_url(UrlKind::Media(uuid));
                uuid
            } else {
                let uuid = state.media.store_url(contact.avatar.clone()).await?;
                contact.avatar = state.config.generate_url(UrlKind::Media(uuid));
                uuid
            };

            state.job_server.queue(CacheMedia::new(uuid)).await?;

            state
                .node_cache
                .set_contact(
                    self.actor_id.clone(),
                    contact.username,
                    contact.display_name,
                    contact.url,
                    contact.avatar,
                )
                .await?;
        }

        let description = ammonia::clean(&description);

        state
            .node_cache
            .set_instance(
                self.actor_id.clone(),
                instance.title,
                description,
                instance.version,
                instance.registrations,
                instance.approval_required,
            )
            .await?;

        Ok(())
    }
}

impl ActixJob for QueryInstance {
    type State = JobState;
    type Future = Pin<Box<dyn Future<Output = Result<(), anyhow::Error>>>>;

    const NAME: &'static str = "relay::jobs::QueryInstance";

    fn run(self, state: Self::State) -> Self::Future {
        Box::pin(async move { self.perform(state).await.map_err(Into::into) })
    }
}

fn default_approval() -> bool {
    false
}

#[derive(serde::Deserialize)]
struct Instance {
    title: String,
    short_description: Option<String>,
    description: String,
    version: String,
    registrations: bool,

    #[serde(default = "default_approval")]
    approval_required: bool,

    #[serde(rename = "contact_account")]
    contact: Option<Contact>,
}

#[derive(serde::Deserialize)]
struct Contact {
    username: String,
    display_name: String,
    url: IriString,
    avatar: IriString,
}
