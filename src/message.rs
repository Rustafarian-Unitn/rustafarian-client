use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use wg_2024::network::NodeId;

/**
 * Represents a message that can be sent between nodes.
 * Contains the source node id, the session id and the content of the message.
 */
#[derive(Debug, Clone)]
pub struct Message<M: DroneSend> {
    pub source_id: NodeId,
    pub session_id: u64,
    pub content: M,
}

/**
 * Serialization/Deserialization of the message
 */
pub trait DroneSend: Serialize + DeserializeOwned {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

pub trait Request: DroneSend {}
pub trait Response: DroneSend {}

/**
 * Request type for a chat client
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRequest {
    ClientList,
    Register(NodeId),
    SendMessage {
        from: NodeId,
        to: NodeId,
        message: String,
    },
}

impl DroneSend for ChatRequest {}
impl Request for ChatRequest {}

/**
 * Response type for a chat client
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatResponse {
    ClientList(Vec<NodeId>),
    MessageFrom { from: NodeId, message: Vec<u8> },
    MessageSent,
}

impl DroneSend for ChatResponse {}
impl Response for ChatResponse {}

/**
 * Server type request (Media, Chat, Text)
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerTypeRequest {
    SendMessage {
        from: NodeId,
        to: NodeId
    },
}

impl DroneSend for ServerTypeRequest {}
impl Request for ServerTypeRequest {}

/**
 * Server type response (Media, Chat, Text)
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerTypeResponse {
    MessageFrom { from: NodeId, server_type: ServerType },
    MessageSent,
}

impl DroneSend for ServerTypeResponse {}
impl Response for ServerTypeResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerType {
    Chat,
    Text,
    Media,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimControllerCommand {
    SendMessage(String, NodeId), // Send message to a server
    Register(NodeId), // Register a client to a server
    ClientList, // Get the list of available clients
}

impl DroneSend for SimControllerCommand {}
impl Request for SimControllerCommand {}