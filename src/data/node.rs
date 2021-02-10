use crate::{
    db::{Contact, Db, Info, Instance},
    error::MyError,
};
use activitystreams::url::Url;
use std::time::{Duration, SystemTime};

#[derive(Clone)]
pub struct NodeCache {
    db: Db,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Node {
    pub(crate) base: Url,
    pub(crate) info: Option<Info>,
    pub(crate) instance: Option<Instance>,
    pub(crate) contact: Option<Contact>,
}

impl NodeCache {
    pub(crate) fn new(db: Db) -> Self {
        NodeCache { db }
    }

    pub(crate) async fn nodes(&self) -> Result<Vec<Node>, MyError> {
        let infos = self.db.connected_info().await?;
        let instances = self.db.connected_instance().await?;
        let contacts = self.db.connected_contact().await?;

        let vec = self
            .db
            .connected_ids()
            .await?
            .into_iter()
            .map(move |actor_id| {
                let info = infos.get(&actor_id).map(|info| info.clone());
                let instance = instances.get(&actor_id).map(|instance| instance.clone());
                let contact = contacts.get(&actor_id).map(|contact| contact.clone());

                Node::new(actor_id)
                    .info(info)
                    .instance(instance)
                    .contact(contact)
            })
            .collect();

        Ok(vec)
    }

    pub(crate) async fn is_nodeinfo_outdated(&self, actor_id: Url) -> bool {
        self.db
            .info(actor_id)
            .await
            .map(|opt| opt.map(|info| info.outdated()).unwrap_or(true))
            .unwrap_or(true)
    }

    pub(crate) async fn is_contact_outdated(&self, actor_id: Url) -> bool {
        self.db
            .contact(actor_id)
            .await
            .map(|opt| opt.map(|contact| contact.outdated()).unwrap_or(true))
            .unwrap_or(true)
    }

    pub(crate) async fn is_instance_outdated(&self, actor_id: Url) -> bool {
        self.db
            .instance(actor_id)
            .await
            .map(|opt| opt.map(|instance| instance.outdated()).unwrap_or(true))
            .unwrap_or(true)
    }

    pub(crate) async fn set_info(
        &self,
        actor_id: Url,
        software: String,
        version: String,
        reg: bool,
    ) -> Result<(), MyError> {
        self.db
            .save_info(
                actor_id,
                Info {
                    software,
                    version,
                    reg,
                    updated: SystemTime::now(),
                },
            )
            .await
    }

    pub(crate) async fn set_instance(
        &self,
        actor_id: Url,
        title: String,
        description: String,
        version: String,
        reg: bool,
        requires_approval: bool,
    ) -> Result<(), MyError> {
        self.db
            .save_instance(
                actor_id,
                Instance {
                    title,
                    description,
                    version,
                    reg,
                    requires_approval,
                    updated: SystemTime::now(),
                },
            )
            .await
    }

    pub(crate) async fn set_contact(
        &self,
        actor_id: Url,
        username: String,
        display_name: String,
        url: Url,
        avatar: Url,
    ) -> Result<(), MyError> {
        self.db
            .save_contact(
                actor_id,
                Contact {
                    username,
                    display_name,
                    url,
                    avatar,
                    updated: SystemTime::now(),
                },
            )
            .await
    }
}

impl Node {
    fn new(mut url: Url) -> Self {
        url.set_fragment(None);
        url.set_query(None);
        url.set_path("");

        Node {
            base: url,
            info: None,
            instance: None,
            contact: None,
        }
    }

    fn info(mut self, info: Option<Info>) -> Self {
        self.info = info;
        self
    }

    fn instance(mut self, instance: Option<Instance>) -> Self {
        self.instance = instance;
        self
    }

    fn contact(mut self, contact: Option<Contact>) -> Self {
        self.contact = contact;
        self
    }
}

static TEN_MINUTES: Duration = Duration::from_secs(60 * 10);

impl Info {
    pub(crate) fn outdated(&self) -> bool {
        self.updated + TEN_MINUTES < SystemTime::now()
    }
}

impl Instance {
    pub(crate) fn outdated(&self) -> bool {
        self.updated + TEN_MINUTES < SystemTime::now()
    }
}

impl Contact {
    pub(crate) fn outdated(&self) -> bool {
        self.updated + TEN_MINUTES < SystemTime::now()
    }
}
