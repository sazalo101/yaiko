//! Deterministic dependency graphs for media-processing pipelines.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaGraphError {
    EmptyId,
    DuplicateNode,
    MissingDependency,
    Cycle,
    Capacity,
    InvalidState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaNode {
    pub id: String,
    pub dependencies: Vec<String>,
    pub state: NodeState,
}

#[derive(Debug, Clone)]
pub struct MediaGraph {
    nodes: BTreeMap<String, MediaNode>,
    max_nodes: usize,
}

impl MediaGraph {
    pub fn new(max_nodes: usize) -> Self {
        Self {
            nodes: BTreeMap::new(),
            max_nodes: max_nodes.max(1),
        }
    }
    pub fn add(
        &mut self,
        id: impl Into<String>,
        dependencies: impl IntoIterator<Item = String>,
    ) -> Result<(), MediaGraphError> {
        let id = id.into();
        if id.is_empty() || id.len() > 128 {
            return Err(MediaGraphError::EmptyId);
        }
        if self.nodes.len() >= self.max_nodes || self.nodes.contains_key(&id) {
            return Err(if self.nodes.len() >= self.max_nodes {
                MediaGraphError::Capacity
            } else {
                MediaGraphError::DuplicateNode
            });
        }
        let dependencies = dependencies.into_iter().collect::<Vec<_>>();
        if dependencies
            .iter()
            .any(|dependency| dependency.is_empty() || dependency == &id)
        {
            return Err(MediaGraphError::MissingDependency);
        }
        self.nodes.insert(
            id.clone(),
            MediaNode {
                id,
                dependencies,
                state: NodeState::Pending,
            },
        );
        Ok(())
    }
    pub fn plan(&self) -> Result<Vec<String>, MediaGraphError> {
        for node in self.nodes.values() {
            for dependency in &node.dependencies {
                if !self.nodes.contains_key(dependency) {
                    return Err(MediaGraphError::MissingDependency);
                }
            }
        }
        let mut indegree = self
            .nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.dependencies.len()))
            .collect::<HashMap<_, _>>();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for node in self.nodes.values() {
            for dependency in &node.dependencies {
                dependents
                    .entry(dependency.clone())
                    .or_default()
                    .push(node.id.clone());
            }
        }
        let mut ready = self
            .nodes
            .iter()
            .filter(|(id, _)| indegree[*id] == 0)
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = ready.pop_first() {
            order.push(id.clone());
            if let Some(children) = dependents.get(&id) {
                for child in children {
                    let count = indegree.get_mut(child).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(MediaGraphError::Cycle);
        }
        Ok(order)
    }
    pub fn mark_running(&mut self, id: &str) -> Result<(), MediaGraphError> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or(MediaGraphError::MissingDependency)?;
        if node.state != NodeState::Pending && node.state != NodeState::Ready {
            return Err(MediaGraphError::InvalidState);
        }
        node.state = NodeState::Running;
        Ok(())
    }
    pub fn mark_succeeded(&mut self, id: &str) -> Result<(), MediaGraphError> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or(MediaGraphError::MissingDependency)?;
        if node.state != NodeState::Running {
            return Err(MediaGraphError::InvalidState);
        }
        node.state = NodeState::Succeeded;
        Ok(())
    }
    pub fn mark_failed(&mut self, id: &str) -> Result<(), MediaGraphError> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or(MediaGraphError::MissingDependency)?;
        if node.state != NodeState::Running {
            return Err(MediaGraphError::InvalidState);
        }
        node.state = NodeState::Failed;
        self.propagate_cancelled(id);
        Ok(())
    }
    pub fn cancel(&mut self, id: &str) -> Result<(), MediaGraphError> {
        if !self.nodes.contains_key(id) {
            return Err(MediaGraphError::MissingDependency);
        }
        self.propagate_cancelled(id);
        Ok(())
    }
    fn propagate_cancelled(&mut self, root: &str) {
        let mut queue = VecDeque::from([root.to_string()]);
        while let Some(id) = queue.pop_front() {
            for node in self.nodes.values_mut() {
                if node.dependencies.iter().any(|dependency| dependency == &id)
                    && matches!(node.state, NodeState::Pending | NodeState::Ready)
                {
                    node.state = NodeState::Cancelled;
                    queue.push_back(node.id.clone());
                }
            }
        }
    }
    pub fn state(&self, id: &str) -> Option<NodeState> {
        self.nodes.get(id).map(|node| node.state)
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_deterministic_topological_plan() {
        let mut graph = MediaGraph::new(8);
        graph
            .add("render", ["captions".into(), "audio".into()])
            .unwrap();
        graph.add("audio", ["source".into()]).unwrap();
        graph.add("captions", ["source".into()]).unwrap();
        graph.add("source", Vec::<String>::new()).unwrap();
        assert_eq!(
            graph.plan().unwrap(),
            vec!["source", "audio", "captions", "render"]
        );
    }
    #[test]
    fn rejects_missing_dependencies_cycles_duplicates_and_capacity() {
        let mut graph = MediaGraph::new(2);
        graph.add("a", ["missing".into()]).unwrap();
        assert_eq!(graph.plan(), Err(MediaGraphError::MissingDependency));
        let mut cycle = MediaGraph::new(3);
        cycle.add("a", ["b".into()]).unwrap();
        cycle.add("b", ["a".into()]).unwrap();
        assert_eq!(cycle.plan(), Err(MediaGraphError::Cycle));
        assert_eq!(
            cycle.add("b", Vec::<String>::new()),
            Err(MediaGraphError::DuplicateNode)
        );
        cycle.add("c", Vec::<String>::new()).unwrap();
        assert_eq!(
            cycle.add("d", Vec::<String>::new()),
            Err(MediaGraphError::Capacity)
        );
    }
    #[test]
    fn propagates_failure_and_cancellation_to_dependents() {
        let mut graph = MediaGraph::new(8);
        graph.add("source", Vec::<String>::new()).unwrap();
        graph.add("render", ["source".into()]).unwrap();
        graph.add("preview", ["render".into()]).unwrap();
        graph.mark_running("source").unwrap();
        graph.mark_failed("source").unwrap();
        assert_eq!(graph.state("render"), Some(NodeState::Cancelled));
        assert_eq!(graph.state("preview"), Some(NodeState::Cancelled));
    }
    #[test]
    fn enforces_lifecycle_transitions() {
        let mut graph = MediaGraph::new(2);
        graph.add("source", Vec::<String>::new()).unwrap();
        assert_eq!(
            graph.mark_succeeded("source"),
            Err(MediaGraphError::InvalidState)
        );
        graph.mark_running("source").unwrap();
        graph.mark_succeeded("source").unwrap();
        assert_eq!(
            graph.mark_running("source"),
            Err(MediaGraphError::InvalidState)
        );
    }
}
