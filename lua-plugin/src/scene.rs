//! Generic scene-node lookup cache owned by the main thread.

use crate::handles::HandleToken;
use fyrox::{
    core::pool::Handle,
    graph::SceneGraph,
    scene::{node::Node, Scene},
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct SceneRegistry {
    nodes: HashMap<String, Handle<Node>>,
}

impl SceneRegistry {
    pub fn resolve(&mut self, scene: &Scene, name: &str) -> Option<Handle<Node>> {
        if let Some(handle) = self.nodes.get(name).copied() {
            if scene.graph.try_get(handle).is_ok() {
                return Some(handle);
            }
            self.nodes.remove(name);
        }
        let (handle, _) = scene.graph.find_by_name_from_root(name)?;
        self.nodes.insert(name.to_owned(), handle);
        Some(handle)
    }

    pub fn token(&mut self, scene: &Scene, name: &str) -> Option<HandleToken> {
        self.resolve(scene, name).map(HandleToken::from_handle)
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}
