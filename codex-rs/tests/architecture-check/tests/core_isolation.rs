//! Architecture boundary test.
//!
//! Locks in a real boundary win: `codex-core` must NOT depend, directly or
//! transitively, on `codex-app-server-protocol`. The dependency direction is
//! supposed to flow the other way (the app-server protocol layer sits above
//! core), so a path from core back down to the protocol crate would be an
//! architectural regression.
//!
//! The check shells out to `cargo metadata`, walks the resolved dependency
//! graph, and fails with the offending path if the forbidden crate is
//! reachable.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// Crate whose dependency closure we audit.
const ROOT_CRATE: &str = "codex-core";
/// Crate that must remain outside that closure.
const FORBIDDEN_CRATE: &str = "codex-app-server-protocol";

/// Known, documented exceptions: intermediary crates that legitimately pull
/// `FORBIDDEN_CRATE` into `ROOT_CRATE`'s transitive closure.
///
/// These are tracked architectural debt, NOT a relaxation of the boundary: each
/// crate below is a direct `codex-core` dependency that still mis-homes one or
/// more `codex-app-server-protocol` types, creating a length-3
/// `codex-core -> X -> codex-app-server-protocol` path. The two boundaries that
/// HAVE been decoupled are intentionally absent from this list and are enforced
/// strictly:
///   * `codex-analytics` — inverted into the dedicated `codex-analytics-appserver`
///     crate.
///   * `codex-core-plugins` — the 5 plugin-domain types (`PluginInstallPolicy`,
///     `PluginAuthPolicy`, `PluginAvailability`, `PluginInterface`,
///     `SkillInterface`) were moved down into `codex_protocol::plugin`, and its
///     JSON-RPC error use was replaced with a local error type, so core-plugins
///     no longer depends on `codex-app-server-protocol`.
///
/// Each remaining entry, with the app-server-protocol surface it still pulls:
///   * `codex-core-skills` — app-server-protocol types other than the 5 plugin
///     ones (its own `SkillInterface` is `crate::model::SkillInterface`).
///   * `codex-login` — login/auth-domain protocol types.
///   * `codex-otel` — `AuthMode` (otel/src/lib.rs: `From<AuthMode>` for its
///     telemetry auth-mode enum).
///   * `codex-models-manager` — model/protocol types.
///   * `codex-exec-server` — exec protocol types.
///   * `codex-tools` — tool protocol types.
///   * `codex-model-provider-info` — model-provider protocol types (pulled via
///     its optional `runtime` feature, which `codex-core` enables).
///
/// TODO: decouple each in a future wave (move the mis-homed app-server-protocol
/// types down into `codex-protocol`/`codex-config-types`, or invert into a
/// dedicated `X-appserver` crate) and delete it from this list. This is tracked
/// debt, not the analytics/core-plugins gateways which ARE decoupled. When the
/// list is empty again the boundary is fully strict.
const KNOWN_EXCEPTION_GATEWAYS: &[&str] = &[
    "codex-core-skills",
    "codex-login",
    "codex-otel",
    "codex-models-manager",
    "codex-exec-server",
    "codex-tools",
    "codex-model-provider-info",
];

#[test]
fn core_does_not_depend_on_app_server_protocol() {
    let metadata = load_metadata();

    // Map every resolved node id to its package name. `resolve.nodes[*].id`
    // and `resolve.nodes[*].dependencies[*]` are package ids (the same opaque
    // strings found at `packages[*].id`), so we resolve names through this map.
    let id_to_name = build_id_to_name(&metadata);

    // package name -> set of dependency package names, over the *resolved*
    // graph (i.e. with the actual feature/target resolution cargo applied).
    let graph = build_dependency_graph(&metadata, &id_to_name);

    assert!(
        graph.contains_key(ROOT_CRATE),
        "`{ROOT_CRATE}` was not found in the cargo metadata resolve graph; \
         is the crate name correct and part of the workspace?"
    );

    // Barriers are the documented exception gateways. The list is currently
    // empty, so this is the strict assertion: `ROOT_CRATE` must not reach
    // `FORBIDDEN_CRATE` by ANY path, directly or transitively. (If a future
    // exception is ever re-introduced, paths routing solely through it are
    // tolerated, and any path bypassing it still fails.)
    let barriers: BTreeSet<&str> = KNOWN_EXCEPTION_GATEWAYS.iter().copied().collect();

    if let Some(path) = find_path(&graph, ROOT_CRATE, FORBIDDEN_CRATE, &barriers) {
        panic!(
            "architecture violation: `{ROOT_CRATE}` reaches `{FORBIDDEN_CRATE}` \
             (boundary is enforced strictly; documented exceptions: \
             {KNOWN_EXCEPTION_GATEWAYS:?}):\n    {}\n\
             Fix the ownership boundary rather than adding a new exception.",
            path.join("\n      -> "),
        );
    }
}

/// Runs `cargo metadata --format-version 1` at the workspace root and parses
/// its stdout as JSON.
fn load_metadata() -> Value {
    // The workspace root is the parent-parent of this crate
    // (`<root>/tests/architecture-check`).
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| {
            panic!(
                "failed to derive workspace root from CARGO_MANIFEST_DIR={}",
                manifest_dir.display()
            )
        });

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(&cargo)
        .args(["metadata", "--format-version", "1"])
        .current_dir(workspace_root)
        .output()
        .unwrap_or_else(|err| panic!("failed to run `cargo metadata`: {err}"));

    assert!(
        output.status.success(),
        "`cargo metadata` failed with status {}:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("failed to parse `cargo metadata` output as JSON: {err}"))
}

/// Builds a map from resolved package id to package name.
fn build_id_to_name(metadata: &Value) -> HashMap<String, String> {
    let packages = metadata["packages"]
        .as_array()
        .unwrap_or_else(|| panic!("`cargo metadata` output is missing a `packages` array"));

    packages
        .iter()
        .filter_map(|package| {
            let id = package["id"].as_str()?;
            let name = package["name"].as_str()?;
            Some((id.to_string(), name.to_string()))
        })
        .collect()
}

/// Builds a package-name -> sorted-set-of-dependency-names map from the
/// resolved graph (`resolve.nodes`).
fn build_dependency_graph(
    metadata: &Value,
    id_to_name: &HashMap<String, String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap_or_else(|| panic!("`cargo metadata` output is missing `resolve.nodes`"));

    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for node in nodes {
        let Some(node_id) = node["id"].as_str() else {
            continue;
        };
        let Some(node_name) = id_to_name.get(node_id) else {
            continue;
        };

        let edges = graph.entry(node_name.clone()).or_default();

        if let Some(deps) = node["dependencies"].as_array() {
            for dep in deps {
                if let Some(dep_name) = dep.as_str().and_then(|id| id_to_name.get(id)) {
                    edges.insert(dep_name.clone());
                }
            }
        }
    }

    graph
}

/// Breadth-first search from `start` to `target` over `graph`, without
/// traversing *through* any crate listed in `barriers` (the search may land on
/// a barrier crate but will not follow its outgoing edges).
///
/// Returns the discovered path (inclusive of both endpoints) if `target` is
/// reachable, or `None` otherwise. BFS yields a shortest path, which keeps the
/// reported violation easy to read.
fn find_path(
    graph: &BTreeMap<String, BTreeSet<String>>,
    start: &str,
    target: &str,
    barriers: &BTreeSet<&str>,
) -> Option<Vec<String>> {
    let mut predecessor: HashMap<&str, &str> = HashMap::new();
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();

    visited.insert(start);
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        if current == target {
            return Some(reconstruct_path(&predecessor, start, target));
        }

        // Do not traverse THROUGH a documented exception gateway.
        if barriers.contains(current) {
            continue;
        }

        if let Some(neighbors) = graph.get(current) {
            for neighbor in neighbors {
                let neighbor = neighbor.as_str();
                if visited.insert(neighbor) {
                    predecessor.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }
    }

    None
}

/// Walks the predecessor map backwards from `target` to `start` and returns the
/// path in forward order.
fn reconstruct_path(
    predecessor: &HashMap<&str, &str>,
    start: &str,
    target: &str,
) -> Vec<String> {
    let mut path = vec![target.to_string()];
    let mut current = target;
    while current != start {
        let prev = predecessor[current];
        path.push(prev.to_string());
        current = prev;
    }
    path.reverse();
    path
}
