use activitystreams::iri_string::types::IriStr;
use std::{collections::HashMap, sync::Mutex};
use time::OffsetDateTime;

pub(crate) struct LastOnline {
    domains: Mutex<HashMap<String, OffsetDateTime>>,
}

impl LastOnline {
    pub(crate) fn mark_seen(&self, iri: &IriStr) {
        if let Some(authority) = iri.authority_str() {
            let mut guard = self.domains.lock().unwrap();
            guard.insert(authority.to_string(), OffsetDateTime::now_utc());
            metrics::gauge!("relay.last-online.size",)
                .set(crate::collector::recordable(guard.len()));
        }
    }

    pub(crate) fn take(&self) -> HashMap<String, OffsetDateTime> {
        std::mem::take(&mut *self.domains.lock().unwrap())
    }

    pub(crate) fn empty() -> Self {
        Self {
            domains: Mutex::new(HashMap::default()),
        }
    }
}
