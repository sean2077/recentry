use recentry_core::RecentProject;

#[derive(Debug, Default)]
pub struct LauncherState {
    projects: Vec<RecentProject>,
    visible: Vec<RecentProject>,
    query: String,
    selected: usize,
}

impl LauncherState {
    pub fn set_projects(&mut self, projects: Vec<RecentProject>) {
        self.projects = projects;
        self.refresh();
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.refresh();
    }

    pub fn reset_query(&mut self) {
        self.set_query("");
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            self.selected = 0;
            return;
        }
        let last = self.visible.len() as isize - 1;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }

    pub fn select_index(&mut self, index: usize) {
        if self.visible.is_empty() {
            self.selected = 0;
        } else {
            self.selected = index.min(self.visible.len() - 1);
        }
    }

    pub fn visible(&self) -> &[RecentProject] {
        &self.visible
    }

    pub fn selected(&self) -> Option<&RecentProject> {
        self.visible.get(self.selected)
    }

    pub fn selected_index(&self) -> Option<usize> {
        (!self.visible.is_empty()).then_some(self.selected)
    }

    fn refresh(&mut self) {
        self.visible = recentry_core::search_projects(&self.projects, &self.query);
        self.selected = 0;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use recentry_core::{ProjectId, ProjectKind, ProjectTarget, ProviderId, RecentProject};

    use super::*;

    fn project(name: &str, recent_index: u32) -> RecentProject {
        RecentProject {
            id: ProjectId(name.to_owned()),
            provider: ProviderId("vscode".to_owned()),
            kind: ProjectKind::Folder,
            target: ProjectTarget::LocalPath(PathBuf::from(format!(r"C:\work\{name}"))),
            name: name.to_owned(),
            detail: format!(r"C:\work\{name}"),
            recent_index,
        }
    }

    #[test]
    fn reset_query_restores_recent_order_and_first_selection() {
        let mut state = LauncherState::default();
        state.set_projects(vec![project("older", 1), project("newer", 0)]);
        state.set_query("older");
        assert_eq!(state.visible()[0].name, "older");
        state.reset_query();
        assert_eq!(
            state
                .visible()
                .iter()
                .map(|project| project.name.as_str())
                .collect::<Vec<_>>(),
            vec!["newer", "older"]
        );
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn selection_is_clamped_to_visible_results() {
        let mut state = LauncherState::default();
        state.set_projects(vec![project("one", 0), project("two", 1)]);
        state.reset_query();
        state.move_selection(10);
        assert_eq!(state.selected().unwrap().name, "two");
        state.set_query("one");
        assert_eq!(state.selected().unwrap().name, "one");
        state.set_query("missing");
        assert!(state.selected().is_none());
    }

    #[test]
    fn direct_selection_is_clamped_to_visible_results() {
        let mut state = LauncherState::default();
        state.set_projects(vec![project("one", 0), project("two", 1)]);
        state.select_index(1);
        assert_eq!(state.selected().unwrap().name, "two");
        state.select_index(99);
        assert_eq!(state.selected().unwrap().name, "two");
    }
}
