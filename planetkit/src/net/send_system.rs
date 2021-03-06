use std;
use std::sync::mpsc::TryRecvError;

use specs;
use specs::{FetchMut};
use slog::Logger;
use futures;

use super::{
    GameMessage,
    WireMessage,
    SendWireMessage,
    SendMessageQueue,
    RecvMessage,
    RecvMessageQueue,
    NewPeer,
    NetworkPeers,
    NetworkPeer,
    Destination,
    PeerId,
    Transport,
};

pub struct SendSystem<G: GameMessage>{
    log: Logger,
    send_udp_tx: futures::sync::mpsc::Sender<SendWireMessage<G>>,
    new_peer_rx: std::sync::mpsc::Receiver<NewPeer<G>>,
}

impl<G> SendSystem<G>
    where G: GameMessage
{
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> SendSystem<G> {
        use auto_resource::AutoResource;

        // Ensure SendMessage ring buffer resource is registered.
        SendMessageQueue::<G>::ensure(world);

        // Ensure NetworkPeers resource is registered.
        NetworkPeers::<G>::ensure(world);

        // Ensure ServerResource is present, and fetch the
        // channel ends we need from it.
        use super::ServerResource;
        let server_resource = ServerResource::<G>::ensure(world);
        let send_udp_tx = server_resource.send_udp_tx.clone();
        let new_peer_rx = server_resource.new_peer_rx
            .lock()
            .expect("Couldn't get lock on new peer receiver")
            .take()
            .expect("Somebody already took it!");

        let system = SendSystem {
            log: parent_log.new(o!()),
            send_udp_tx: send_udp_tx,
            new_peer_rx: new_peer_rx,
        };
        system
    }

    fn send_message(&mut self, game_message: G, dest_peer: &mut NetworkPeer<G>, transport: Transport) {
        // Decide whether the message should go over TCP or UDP.
        match transport {
            Transport::UDP => {
                // Look up the destination socket address for this peer.
                // (Peer ID 0 refers to self, and isn't in the vec.)
                let dest_socket_addr = dest_peer.socket_addr;

                // Re-wrap the message for sending.
                let send_wire_message = SendWireMessage {
                    dest: dest_socket_addr,
                    message: WireMessage::Game(game_message),
                };

                self.send_udp_tx.try_send(send_wire_message).unwrap_or_else(|err| {
                    error!(self.log, "Could send message to UDP client; was the buffer full?"; "err" => format!("{:?}", err));
                    ()
                });
            },
            Transport::TCP => {
                // Look up TCP sender channel for this peer.
                // (Peer ID 0 refers to self, and isn't in the vec.)
                let sender = &mut dest_peer.tcp_sender;

                let wire_message = WireMessage::Game(game_message);
                sender.try_send(wire_message).unwrap_or_else(|err| {
                    error!(self.log, "Could send message to TCP client; was the buffer full?"; "err" => format!("{:?}", err));
                    ()
                });
            }
        }
    }
}

impl<'a, G> specs::System<'a> for SendSystem<G>
    where G: GameMessage
{
    // TODO: require peer list as systemdata.
    type SystemData = (
        FetchMut<'a, SendMessageQueue<G>>,
        FetchMut<'a, RecvMessageQueue<G>>,
        FetchMut<'a, NetworkPeers<G>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut send_message_queue,
            mut recv_message_queue,
            mut network_peers,
        ) = data;

        // TODO: does this stuff even belong here,
        // or should things like `send_system_new_peer_sender`
        // be in RESOURCES? I think the latter.
        // Then you can have a NewPeerSystem or something.
        // TODO: try that.

        // Register any new peers that have connected
        // (or that we've connected to).
        loop {
            match self.new_peer_rx.try_recv() {
                Ok(new_peer) => {
                    // Peer ID 0 refers to self, and isn't in the array.
                    let next_peer_id = PeerId(network_peers.peers.len() as u16 + 1);
                    let peer = NetworkPeer {
                        id: next_peer_id,
                        tcp_sender: new_peer.tcp_sender,
                        socket_addr: new_peer.socket_addr,
                    };
                    network_peers.peers.push(peer);

                    // Leave a note about the new peer so game-specific
                    // systems can do whatever initialization they might
                    // need to do.
                    network_peers.new_peers.push_back(next_peer_id);
                },
                Err(err) => {
                    match err {
                        TryRecvError::Empty => {
                            break;
                        },
                        TryRecvError::Disconnected => {
                            // TODO: don't panic; we're going to need
                            // a way to shut the server down gracefully.
                            panic!("Sender hung up");
                        },
                    }
                },
            }
        }

        // Send everything in send queue to UDP/TCP server.
        while let Some(message) = send_message_queue.queue.pop_front() {
            // Re-wrap message and send it to its destination(s).
            match message.destination {
                Destination::One(peer_id) => {
                    // If the destination is ourself,
                    // then just put it straight back on the recv
                    // message queue. This is useful because being
                    // able to treat yourself as just another client/peer
                    // sometimes allows for more general code, rather than
                    // specialising between client or server case.
                    if peer_id.0 == 0 {
                        recv_message_queue.queue.push_back(
                            RecvMessage {
                                game_message: message.game_message,
                            }
                        );
                    } else {
                        self.send_message(
                            message.game_message,
                            &mut network_peers.peers[peer_id.0 as usize - 1],
                            message.transport,
                        );
                    }
                },
                Destination::EveryoneElse => {
                    for peer in network_peers.peers.iter_mut() {
                        self.send_message(
                            message.game_message.clone(),
                            peer,
                            message.transport,
                        );
                    }
                }
            }
        }
    }
}
