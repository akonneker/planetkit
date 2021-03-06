mod recv_system;
mod send_system;
mod server;
mod server_resource;
mod udp;
mod tcp;

#[cfg(test)]
mod tests;

use std::fmt::Debug;
use std::net::SocketAddr;
use std::collections::vec_deque::VecDeque;

use serde::Serialize;
use serde::de::DeserializeOwned;
use futures;
use specs;

use ::AutoResource;
pub use self::recv_system::RecvSystem;
pub use self::send_system::SendSystem;
pub use self::server::Server;
pub use self::server_resource::ServerResource;

// TODO: all this naming is pretty shoddy, and evolved in an awkward
// way that makes it super unclear what's for what.

// Game-specific message body.
//
// These are forwarded to systems without any filtering or sanitization
// by generic network systems. Therefore they should in general be treated
// as a verbatim message from a peer that is only trusted as much as that
// peer is trusted.
//
// Exists primarily as a way to aggregate all the super-traits we expect,
// especially around being able to serialize it.
pub trait GameMessage : 'static + Serialize + DeserializeOwned + Debug + Eq + PartialEq + Send + Sync + Clone {}

// TODO: identify self in every message. Make this a struct wrapping the enum,
// or include your identity in Goodbye and a Game wrapper?
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum WireMessage<G> {
    /// First message you should send to any peer when establishing a connection
    /// (keeping in mind that this is only a logical connection in PlanetKit, not a stateful TCP connection)
    /// regardless of the roles each peer might have (server, client, equal).
    Hello,
    /// Courtesy message before disconnecting, so that your peer can regard
    /// you as having cleanly disconnected rather than mysteriously disappearing.
    Goodbye,
    /// Game-specific message, opaque to PlanetKit aside from the constraints
    /// placed on it by `GameMessage`.
    Game(G),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RecvWireMessage<G> {
    src: SocketAddr,
    // TODO: error type for mangled message
    message: Result<WireMessage<G>, ()>,
}

// Only actually used for UDP; for TCP messages there are
// per-peer channels all the way to the SendSystem, so there's
// no need for an extra envelope around the message.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SendWireMessage<G> {
    dest: SocketAddr,
    message: WireMessage<G>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RecvMessage<G> {
    // TODO: sender peer id
    pub game_message: G,
}

#[derive(Debug, Clone, Copy)]
pub enum Transport {
    UDP,
    TCP,
}

/// Game message wrapped for sending to peer(s).
/// Might wrap a module's message, or a game's message.
#[derive(Debug)]
pub struct SendMessage<G> {
    pub destination: Destination,
    pub game_message: G,
    /// The network transport that should be used to send this message.
    pub transport: Transport,
}

#[derive(Debug)]
pub enum Destination {
    One(PeerId),
    EveryoneElse,
    // TODO: consider adding a new one for "all including self"
    // so we can simplify some code paths that work differently
    // depending on whether you're the server or not.
}

/// `World`-global resource for game messages waiting to be dispatched
/// to game-specific systems.
pub struct RecvMessageQueue<G> {
    pub queue: VecDeque<RecvMessage<G>>,
}

/// `World`-global resource for game messages waiting to be sent
/// to peers.
pub struct SendMessageQueue<G> {
    pub queue: VecDeque<SendMessage<G>>,
}

impl<G: GameMessage> AutoResource for SendMessageQueue<G> {
    fn new(_world: &mut specs::World) -> SendMessageQueue<G> {
        SendMessageQueue {
            queue: VecDeque::<SendMessage<G>>::new(),
        }
    }
}

/// Local identifier for a network peer.
///
/// This identifier is used to label network peers
/// within this host; i.e. it should never be communicated
/// to a peer.
///
/// Note that this is not the same thing as a player ID.
/// This is used in deciding what network peer to send
/// messages to, and which peers have authority over what
/// parts of the world. We might receive messages regarding
/// multiple players from one peer, and need to decide
/// whether that peer has authority to make assertions about
/// those players.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct PeerId(pub u16);

/// A new network peer.
///
/// Might be a server we connected to,
/// or a client that connected to us.
/// Contains the peer's address, and a channel used
/// to send it message over TCP.
///
/// This is used to communicate these essentials
/// to the `SendSystem` when a new connection is established.
pub struct NewPeer<G> {
    pub tcp_sender: futures::sync::mpsc::Sender<WireMessage<G>>,
    pub socket_addr: SocketAddr,
}

pub struct NetworkPeer<G> {
    pub id: PeerId,
    pub tcp_sender: futures::sync::mpsc::Sender<WireMessage<G>>,
    pub socket_addr: SocketAddr,
    // TODO: connection state, etc.
}

/// `World`-global resource for network peers.
pub struct NetworkPeers<G> {
    pub peers: Vec<NetworkPeer<G>>,
    // List of new peers for a single game-specific
    // system to use.
    // TODO: This makes yet another good use case for some kind
    // of pub/sub event system.
    pub new_peers: VecDeque<PeerId>,
}

impl<G: GameMessage> AutoResource for NetworkPeers<G> {
    fn new(_world: &mut specs::World) -> NetworkPeers<G> {
        NetworkPeers {
            peers: Vec::<NetworkPeer<G>>::new(),
            new_peers: VecDeque::<PeerId>::new(),
        }
    }
}
