pub mod generator;
pub mod grid;

#[derive(thiserror::Error, Debug)]
#[error("Failed to generate, contradiction at node with index {}", node_index)]
pub struct GenerationError {
    pub node_index: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum RulesError {
    #[error("Empty models or sockets collection")]
    NoModelsOrSockets,
}

#[cfg(test)]
mod tests {}
