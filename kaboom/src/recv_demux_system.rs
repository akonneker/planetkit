use specs;
use specs::{FetchMut};
use slog::Logger;

use pk::cell_dweller;
use pk::net::{
    RecvMessage,
    RecvMessageQueue,
};

use ::message::Message;
use ::player;

pub struct RecvDemuxSystem{
    log: Logger,
}

impl RecvDemuxSystem {
    pub fn new(parent_log: &Logger, _world: &mut specs::World) -> RecvDemuxSystem {
        RecvDemuxSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for RecvDemuxSystem {
    type SystemData = (
        FetchMut<'a, RecvMessageQueue<Message>>,
        FetchMut<'a, cell_dweller::RecvMessageQueue>,
        FetchMut<'a, player::RecvMessageQueue>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut recv_message_queue,
            mut cell_dweller_recv_queue,
            mut player_recv_queue,
        ) = data;

        // Drain the recv message queue, and dispatch to system-specific queues.
        while let Some(message) = recv_message_queue.queue.pop_front() {
            match message.game_message {
                Message::CellDweller(cd_message) => {
                    trace!(self.log, "Forwarding cell dweller message to its recv message queue"; "message" => format!("{:?}", cd_message));
                    cell_dweller_recv_queue.queue.push_back(
                        RecvMessage {
                            game_message: cd_message,
                        }
                    );
                },
                Message::Player(player_message) => {
                    trace!(self.log, "Forwarding player message to its recv message queue"; "message" => format!("{:?}", player_message));
                    player_recv_queue.queue.push_back(
                        RecvMessage {
                            game_message: player_message,
                        }
                    );
                },
            }
        }
    }
}
