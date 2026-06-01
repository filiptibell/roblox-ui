use rbx_dom_weak::types::Ref;
use serde::Serialize;
use ustr::Ustr;

use roblox_ui_project::Delta;

/**
    The stdio notification shape consumed by the VS Code extension.

    The extension treats the tree as pull-based: a notification says "something
    under here was added / removed / changed — re-fetch it". We therefore map the
    richer self-contained [`Delta`] stream down onto this stable schema.
*/
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum DomNotification {
    Changed {
        id: Ref,
        #[serde(skip_serializing_if = "Option::is_none", rename = "className")]
        class_name: Option<Ustr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Added {
        #[serde(skip_serializing_if = "Option::is_none", rename = "parentId")]
        parent_id: Option<Ref>,
        #[serde(rename = "childId")]
        child_id: Ref,
    },
    Removed {
        #[serde(skip_serializing_if = "Option::is_none", rename = "parentId")]
        parent_id: Option<Ref>,
        #[serde(rename = "childId")]
        child_id: Ref,
    },
}

fn parent_opt(parent: Ref) -> Option<Ref> {
    if parent.is_none() {
        None
    } else {
        Some(parent)
    }
}

/**
    Map a single delta to the extension-facing notification.
*/
pub fn delta_to_notification(delta: &Delta) -> DomNotification {
    match delta {
        Delta::Added { id, parent, .. } => DomNotification::Added {
            parent_id: parent_opt(*parent),
            child_id: *id,
        },
        Delta::Removed { id, parent } => DomNotification::Removed {
            parent_id: parent_opt(*parent),
            child_id: *id,
        },
        Delta::Renamed { id, name } => DomNotification::Changed {
            id: *id,
            class_name: None,
            name: Some(name.clone()),
        },
        Delta::Reclassed { id, class } => DomNotification::Changed {
            id: *id,
            class_name: Some(*class),
            name: None,
        },
        Delta::PropertiesChanged { id, .. } | Delta::MetadataChanged { id } => {
            DomNotification::Changed {
                id: *id,
                class_name: None,
                name: None,
            }
        }
    }
}
