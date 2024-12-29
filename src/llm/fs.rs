// use anyhow::Result;
// use std::net::SocketAddr;

// #[derive(Clone, Debug)]
// pub struct LLMWithFS {
//     addr: SocketAddr,
//     allowed_paths: Vec<String>,
// }

// impl LLMWithFS {
//     pub async fn new(addr: SocketAddr, allowed_paths: Vec<String>) -> Result<Self> {
//         Ok(Self {
//             addr,
//             allowed_paths,
//         })
//     }

//     pub async fn chat(&self, message: &str) -> Result<String> {
//         // Implement your chat logic here
//         Ok(format!("Response to: {}", message))
//     }
// }
