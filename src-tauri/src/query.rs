//! Instant search over the scanned tree: name text (substring or `*`/`?` glob), size, kind and
//! "untouched for N months". It reads only the in-memory tree, so it never touches the disk and
//! never offers anything the scan did not track (small files stay folded into their folder).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Kind, Node};

pub const MAX_LIMIT: usize = 5000;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Query {
    /// Case-insensitive. A `*` or `?` makes it a glob over the whole name; otherwise a substring.
    pub text: String,
    pub min_size: u64,
    /// Seconds since the last change; 0 disables the age filter. Applies to files only, because a
    /// folder's own mtime says nothing about the files inside it.
    pub older_than_secs: u64,
    pub kind: Option<Kind>,
    pub include_dirs: bool,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: u64,
    pub kind: Kind,
    pub is_dir: bool,
}

/// `*` matches any run (including none), `?` exactly one character. Case-insensitive.
#[cfg(test)]
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    glob_match_chars(&p, &n)
}

fn glob_match_chars(p: &[char], n: &[char]) -> bool {
    let (mut pi, mut ni) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ni));
            pi += 1;
        } else if let Some((sp, sn)) = star {
            pi = sp + 1;
            ni = sn + 1;
            star = Some((sp, sn + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

fn contains_ignore_case(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        let needle = needle_lower.as_bytes();
        haystack
            .as_bytes()
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
    } else {
        haystack.to_lowercase().contains(needle_lower)
    }
}

struct Matcher<'a> {
    q: &'a Query,
    text: String,
    glob: Option<Vec<char>>,
    cutoff: Option<u64>,
}

impl Matcher<'_> {
    fn accepts(&self, node: &Node, scratch: &mut Vec<char>) -> bool {
        if node.size < self.q.min_size || self.q.kind.is_some_and(|k| node.kind != k) {
            return false;
        }
        if let Some(cutoff) = self.cutoff {
            if node.is_dir || node.modified == 0 || node.modified > cutoff {
                return false;
            }
        }
        if self.text.is_empty() {
            return true;
        }
        if let Some(pattern) = &self.glob {
            scratch.clear();
            scratch.extend(node.name.chars().flat_map(char::to_lowercase));
            glob_match_chars(pattern, scratch)
        } else {
            contains_ignore_case(&node.name, &self.text)
        }
    }
}

/// Matches of `q` in the tree, largest first, at most `q.limit` (default 200, max 5000).
/// `keep` drops OS-owned paths before the cut, like the biggest-files list.
pub fn search(
    tree: &Node,
    root: &Path,
    q: &Query,
    now: u64,
    keep: impl Fn(&Path) -> bool,
) -> Vec<Hit> {
    let limit = if q.limit == 0 { 200 } else { q.limit }.min(MAX_LIMIT);
    let text = q.text.trim().to_lowercase();
    let glob = text.contains(['*', '?']).then(|| text.chars().collect());
    let m = Matcher {
        q,
        glob,
        text,
        cutoff: (q.older_than_secs > 0).then(|| now.saturating_sub(q.older_than_secs)),
    };
    let mut state = WalkState::default();
    walk(tree, root.to_path_buf(), true, &m, &keep, limit, &mut state);
    sort_and_cut(&mut state.hits, limit);
    state.hits
}

#[derive(Default)]
struct WalkState {
    scratch: Vec<char>,
    hits: Vec<Hit>,
    /// Size of the last hit kept by a full cut: a smaller subtree cannot enter the result.
    floor: u64,
}

fn sort_and_cut(hits: &mut Vec<Hit>, limit: usize) {
    hits.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    hits.truncate(limit);
}

fn walk(
    node: &Node,
    path: PathBuf,
    is_root: bool,
    m: &Matcher,
    keep: &dyn Fn(&Path) -> bool,
    limit: usize,
    state: &mut WalkState,
) {
    if node.grouped {
        return;
    }
    let listed = !is_root && (!node.is_dir || m.q.include_dirs);
    if listed && m.accepts(node, &mut state.scratch) && keep(&path) {
        state.hits.push(Hit {
            path: path.to_string_lossy().into_owned(),
            name: node.name.clone(),
            size: node.size,
            modified: node.modified,
            kind: node.kind,
            is_dir: node.is_dir,
        });
        // Amortized bound: memory stays O(limit) on a multi-million-file scan.
        if state.hits.len() >= limit * 4 {
            sort_and_cut(&mut state.hits, limit);
            state.floor = state.hits.last().map_or(0, |h| h.size);
        }
    }
    for c in &node.children {
        // A folder's size bounds everything below it, so whole subtrees are skipped unvisited.
        if c.size < m.q.min_size || c.size < state.floor {
            continue;
        }
        walk(c, path.join(&c.name), false, m, keep, limit, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, size: u64, modified: u64, kind: Kind) -> Node {
        Node::file(name.into(), size, modified, kind)
    }

    fn dir(name: &str, children: Vec<Node>) -> Node {
        Node {
            name: name.into(),
            size: children.iter().map(|c| c.size).sum(),
            files: children.len() as u64,
            modified: 1,
            kind: Kind::Other,
            is_dir: true,
            grouped: false,
            reclaimable: false,
            children,
        }
    }

    fn tree() -> Node {
        dir(
            "/r",
            vec![
                file("Movie.MKV", 900, 100, Kind::Media),
                file("notes.txt", 5, 900, Kind::Documents),
                dir(
                    "src",
                    vec![
                        file("main.rs", 50, 950, Kind::Code),
                        file("old.zip", 400, 50, Kind::Archives),
                    ],
                ),
            ],
        )
    }

    fn run(q: Query) -> Vec<String> {
        search(&tree(), Path::new("/r"), &q, 1000, |_| true)
            .into_iter()
            .map(|h| h.name)
            .collect()
    }

    #[test]
    fn glob_handles_stars_and_marks() {
        assert!(glob_match("*.mkv", "Movie.MKV"));
        assert!(glob_match("m?in.rs", "main.rs"));
        assert!(glob_match("*", ""));
        assert!(glob_match("a*b*c", "aXXbYYc"));
        assert!(!glob_match("*.mkv", "movie.mkv.part"));
        assert!(!glob_match("?", ""));
    }

    #[test]
    fn text_is_a_case_insensitive_substring_without_wildcards() {
        let q = Query {
            text: "MOVIE".into(),
            ..Default::default()
        };
        assert_eq!(run(q), ["Movie.MKV"]);
    }

    #[test]
    fn results_are_largest_first_and_limited() {
        let all = run(Query::default());
        assert_eq!(all, ["Movie.MKV", "old.zip", "main.rs", "notes.txt"]);
        let two = run(Query {
            limit: 2,
            ..Default::default()
        });
        assert_eq!(two, ["Movie.MKV", "old.zip"]);
    }

    #[test]
    fn size_and_kind_filters_combine() {
        let q = Query {
            min_size: 100,
            kind: Some(Kind::Archives),
            ..Default::default()
        };
        assert_eq!(run(q), ["old.zip"]);
    }

    #[test]
    fn age_filter_keeps_only_files_untouched_for_long() {
        // now = 1000, older than 500s => modified <= 500.
        let q = Query {
            older_than_secs: 500,
            ..Default::default()
        };
        assert_eq!(run(q), ["Movie.MKV", "old.zip"]);
    }

    #[test]
    fn age_filter_never_lists_folders() {
        let q = Query {
            older_than_secs: 500,
            include_dirs: true,
            ..Default::default()
        };
        assert!(!run(q).contains(&"src".to_string()));
    }

    #[test]
    fn folders_appear_only_on_request() {
        assert!(!run(Query::default()).contains(&"src".to_string()));
        let with = run(Query {
            include_dirs: true,
            text: "src".into(),
            ..Default::default()
        });
        assert_eq!(with, ["src"]);
    }

    #[test]
    fn rejected_paths_are_dropped_before_the_cut() {
        let q = Query {
            limit: 1,
            ..Default::default()
        };
        let hits = search(&tree(), Path::new("/r"), &q, 1000, |p| {
            !p.ends_with("Movie.MKV")
        });
        assert_eq!(hits[0].name, "old.zip");
    }

    #[test]
    fn grouped_buckets_are_never_hits() {
        let mut t = tree();
        let mut g = file("12 small files", 9999, 1, Kind::Other);
        g.grouped = true;
        t.children.push(g);
        let hits = search(&t, Path::new("/r"), &Query::default(), 1000, |_| true);
        assert!(hits.iter().all(|h| h.name != "12 small files"));
    }

    #[test]
    fn unicode_substring_and_glob_are_case_insensitive() {
        let mut t = tree();
        t.children
            .push(file("Relatório_FINAL.PDF", 300, 100, Kind::Documents));
        let sub = search(
            &t,
            Path::new("/r"),
            &Query {
                text: "RELATÓRIO".into(),
                ..Default::default()
            },
            1000,
            |_| true,
        );
        assert_eq!(sub.len(), 1);
        assert_eq!(sub[0].name, "Relatório_FINAL.PDF");
        let glob = search(
            &t,
            Path::new("/r"),
            &Query {
                text: "relatório_*.pdf".into(),
                ..Default::default()
            },
            1000,
            |_| true,
        );
        assert_eq!(glob.len(), 1);
        assert_eq!(glob[0].name, "Relatório_FINAL.PDF");
    }

    #[test]
    fn subtrees_that_cannot_beat_the_cut_are_never_visited() {
        // Four big files fill the first cut; a whole-drive search must not then walk every small file.
        let big: Vec<Node> = (0..4)
            .map(|i| file(&format!("big{i}"), 1000, 1, Kind::Other))
            .collect();
        let small: Vec<Node> = (0..100)
            .map(|i| file(&format!("s{i}"), 1, 1, Kind::Other))
            .collect();
        let t = dir("/r", vec![dir("a", big), dir("z", small)]);
        let visited = std::cell::Cell::new(0);
        let q = Query {
            limit: 1,
            ..Default::default()
        };
        let hits = search(&t, Path::new("/r"), &q, 1000, |_| {
            visited.set(visited.get() + 1);
            true
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(visited.get(), 4);
    }

    #[test]
    fn a_subtree_as_big_as_the_cut_is_still_visited_for_the_path_tie_break() {
        let tied: Vec<Node> = (0..4)
            .map(|i| file(&format!("f{i}"), 10, 1, Kind::Other))
            .collect();
        let t = dir(
            "/r",
            vec![
                dir("b", tied),
                dir("a", vec![file("x", 10, 1, Kind::Other)]),
            ],
        );
        let q = Query {
            limit: 1,
            ..Default::default()
        };
        let hits = search(&t, Path::new("/r"), &q, 1000, |_| true);
        assert_eq!(
            hits[0].path,
            Path::new("/r").join("a").join("x").to_string_lossy()
        );
    }
}
