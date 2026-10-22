//! A job is either a context-shape computation or a docking run.

use serde::{Deserialize, Serialize};

use crate::docking::DockSpec;
use crate::pipeline::Spec;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Task {
    Shapes(Spec),
    Dock(DockSpec),
}

impl Task {
    /// Jobs stored before docking existed have a bare context-shape spec.
    pub fn from_value(v: serde_json::Value) -> serde_json::Result<Task> {
        match serde_json::from_value::<Task>(v.clone()) {
            Ok(t) => Ok(t),
            Err(e) => serde_json::from_value::<Spec>(v)
                .map(Task::Shapes)
                .map_err(|_| e),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Task::Shapes(s) => &s.name,
            Task::Dock(d) => &d.name,
        }
    }

    /// Short label stored with the job and shown in lists: `corrected`, `legacy` or `dock`.
    pub fn label(&self) -> String {
        match self {
            Task::Shapes(s) => serde_json::to_value(s.mode)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default(),
            Task::Dock(_) => "dock".into(),
        }
    }
}
