//! Go to Agent: the agents whose name or project contains the query, then the Settings pages,
//! as in Saddle's Search (`src/search.rs` at commit `df1c727`); plugins come later.
use crate::{corral::Agent, settings::Page};

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Agent(String),
    Settings(Page),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub target: Target,
    pub label: String,
    /// The agent's project; empty for a Settings page.
    pub detail: String,
}

/// The last part of the agent's working directory.
fn project(agent: &Agent) -> &str {
    let cwd = agent.cwd.as_deref().unwrap_or("").trim_end_matches('/');
    cwd.rsplit('/').next().unwrap_or(cwd)
}

/// Agents whose name or project contains `query`, ignoring case, by name; then the Settings
/// pages whose `Settings › Page` contains it.
pub fn entries(agents: &[Agent], query: &str) -> Vec<Entry> {
    let query = query.trim().to_lowercase();
    let mut matching: Vec<&Agent> = agents
        .iter()
        .filter(|a| {
            a.name.to_lowercase().contains(&query) || project(a).to_lowercase().contains(&query)
        })
        .collect();
    matching.sort_by(|a, b| a.name.cmp(&b.name));
    let mut entries: Vec<Entry> = matching
        .into_iter()
        .map(|a| Entry {
            target: Target::Agent(a.name.clone()),
            label: a.name.clone(),
            detail: project(a).to_owned(),
        })
        .collect();
    entries.extend(Page::ALL.into_iter().filter_map(|page| {
        let label = format!("Settings › {}", page.label());
        label.to_lowercase().contains(&query).then(|| Entry {
            target: Target::Settings(page),
            label,
            detail: String::new(),
        })
    }));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str, cwd: &str) -> Agent {
        Agent {
            name: name.into(),
            cwd: Some(cwd.into()),
            ..Default::default()
        }
    }

    fn labels(entries: &[Entry]) -> Vec<&str> {
        entries.iter().map(|e| e.label.as_str()).collect()
    }

    #[test]
    fn name_or_project_ignoring_case_by_name() {
        let agents = [
            agent("web/main", "/code/Shop"),
            agent("api/review", "/code/api/"),
            agent("api/main", "/code/api"),
        ];
        assert_eq!(labels(&entries(&agents, "SHOP")), ["web/main"]);
        assert_eq!(
            labels(&entries(&agents, " api ")),
            ["api/main", "api/review"]
        );
        assert_eq!(labels(&entries(&agents, "review")), ["api/review"]);
        assert_eq!(entries(&agents, "shop")[0].detail, "Shop");
        assert_eq!(
            entries(&agents, "web")[0].target,
            Target::Agent("web/main".into())
        );
    }

    #[test]
    fn settings_pages_follow_the_agents() {
        let agents = [agent("colors/main", "/code/x")];
        let found = entries(&agents, "colors");
        assert_eq!(labels(&found), ["colors/main", "Settings › Colors"]);
        assert_eq!(found[1].target, Target::Settings(Page::Colors));
        // An empty query lists everything.
        assert_eq!(entries(&agents, "").len(), 4);
        assert_eq!(labels(&entries(&agents, "settings")).len(), 3);
    }
}
