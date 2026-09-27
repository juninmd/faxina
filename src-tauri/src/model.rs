use serde::Serialize;

/// Semantic bucket used for coloring and for deciding what is safe to reclaim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Cache,
    Build,
    Git,
    Code,
    Media,
    Documents,
    Archives,
    Apps,
    Other,
}

impl Kind {
    /// Regenerable content: deleting it costs time, never data.
    pub fn reclaimable(self) -> bool {
        matches!(self, Kind::Cache | Kind::Build)
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub size: u64,
    pub files: u64,
    pub modified: u64,
    pub kind: Kind,
    pub is_dir: bool,
    /// Synthetic bucket of files too small to track individually; not addressable.
    pub grouped: bool,
    /// Set only by name rules (or inherited from a matching ancestor), never by color:
    /// a folder that is *mostly* build output still holds source code.
    pub reclaimable: bool,
    pub children: Vec<Node>,
}

impl Node {
    pub fn file(name: String, size: u64, modified: u64, kind: Kind) -> Self {
        Node {
            name,
            size,
            files: 1,
            modified,
            kind,
            is_dir: false,
            grouped: false,
            reclaimable: kind.reclaimable(),
            children: Vec::new(),
        }
    }

    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name && !c.grouped)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub files: u64,
    pub modified: u64,
    pub kind: Kind,
    pub is_dir: bool,
    pub grouped: bool,
    pub reclaimable: bool,
    pub children: Vec<ViewNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub kind: Kind,
    pub reason: String,
}
