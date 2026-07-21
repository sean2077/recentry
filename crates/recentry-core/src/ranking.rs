use std::collections::HashSet;

use crate::{ProjectTarget, RecentProject};

pub fn deduplicate(mut projects: Vec<RecentProject>) -> Vec<RecentProject> {
    projects.sort_by_key(|project| project.recent_index);
    let mut seen = HashSet::with_capacity(projects.len());
    projects
        .into_iter()
        .filter(|project| seen.insert(target_key(&project.target)))
        .collect()
}

pub fn search_projects(projects: &[RecentProject], query: &str) -> Vec<RecentProject> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        let mut ordered = projects.to_vec();
        ordered.sort_by_key(|project| project.recent_index);
        return ordered;
    }

    let mut scored = projects
        .iter()
        .filter_map(|project| score(project, &query).map(|score| (score, project.clone())))
        .collect::<Vec<_>>();
    scored.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.recent_index.cmp(&right.recent_index))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    scored.into_iter().map(|(_, project)| project).collect()
}

pub fn target_key(target: &ProjectTarget) -> String {
    match target {
        ProjectTarget::LocalPath(path) => {
            let normalized = path
                .to_string_lossy()
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_lowercase();
            format!("path:{normalized}")
        }
        ProjectTarget::Uri(uri) => format!("uri:{}", uri.trim_end_matches('/')),
    }
}

fn score(project: &RecentProject, query: &str) -> Option<i64> {
    let name = project.name.to_lowercase();
    let detail = project.detail.to_lowercase();
    let mut score = if name == query {
        10_000
    } else if name.starts_with(query) {
        7_500
    } else if name.contains(query) {
        5_000
    } else if detail.contains(query) {
        3_000
    } else if fuzzy_match(&name, query) {
        1_500
    } else if fuzzy_match(&detail, query) {
        1_000
    } else {
        return None;
    };
    score += query.chars().count() as i64 * 10;
    Some(score)
}

fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let mut needle = needle.chars();
    let Some(mut expected) = needle.next() else {
        return true;
    };
    for character in haystack.chars() {
        if character == expected {
            match needle.next() {
                Some(next) => expected = next,
                None => return true,
            }
        }
    }
    false
}
