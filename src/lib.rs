mod mutate;

use std::cell::Cell;
use wasm_bindgen::prelude::*;
use tf_demo_parser::{Demo, DemoParser};
use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::parser::{RawPacketStream, DemoHandler, Encode};
use tf_demo_parser::demo::packet::{Packet, PacketType};
use tf_demo_parser::demo::message::Message;
use bitbuffer::{BitWriteStream, LittleEndian, BitRead, BitWrite};
use tf_demo_parser::demo::message::packetentities::{EntityId, PacketEntitiesMessage, PacketEntity, UpdateType};
use tf_demo_parser::demo::message::usermessage::UserMessageType;
use tf_demo_parser::demo::packet::datatable::ClassId;
use tf_demo_parser::demo::parser::analyser::Team;
use tf_demo_parser::demo::sendprop::{SendProp, SendPropIdentifier, SendPropValue};
use crate::mutate::{MessageMutator, MutatorList, PacketMutator};

extern crate web_sys;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}


#[wasm_bindgen]
pub fn unlock(input: &[u8]) -> Vec<u8> {
    set_panic_hook();
    let mut out_buffer = Vec::with_capacity(input.len());
    {
        let mut out_stream = BitWriteStream::new(&mut out_buffer, LittleEndian);

        let demo = Demo::new(&input);
        let spectator_id = find_stv(&demo).expect("no stv bot found");
        dbg!(spectator_id);

        let mut stream = demo.get_stream();
        let header = Header::read(&mut stream).unwrap();
        header.write(&mut out_stream).unwrap();

        let mut packets = RawPacketStream::new(stream.clone());
        let mut handler = DemoHandler::default();
        handler.handle_header(&header);

        let mut mutators = MutatorList::new();
        mutators.push_message_mutator(move |message: &mut Message| {
            if let Message::ServerInfo(info) = message {
                info.player_slot = u32::from(spectator_id) as u8 - 1;
            }
        });
        mutators.push_message_filter(|message: &Message| {
            !matches!(message, Message::SetView(_))
        });
        mutators.push_message_mutator(|message: &mut Message| {
            if let Message::ServerInfo(info) = message {
                info.stv = true;
            }
        });
        mutators.push_packet_mutator(|packet: &mut Packet| {
            if let Packet::Message(message_packet) = packet {
                message_packet.meta.view_angles = Default::default();
            };
        });
        mutators.push_message_filter(|message: &Message| {
            if let Message::UserMessage(usr_message) = message {
                UserMessageType::CloseCaption != usr_message.message_type()
            } else {
                true
            }
        });
        mutators.push_message_mutator(AddStvEntity::new(spectator_id));
        // 1794


        while let Some(mut packet) = packets.next(&handler.state_handler).unwrap() {
            let tick = packet.tick();
            mutators.mutate_packet(&mut packet);

            if packet.packet_type() != PacketType::ConsoleCmd && packet.packet_type() != PacketType::UserCmd {
                packet
                    .encode(&mut out_stream, &handler.state_handler)
                    .unwrap();
            }
            handler.handle_packet(packet).unwrap();

            // if tick > 10_000 {
            //     break;
            //     todo!()
            // }
        }
    }
    out_buffer
}

struct AddStvEntity {
    added: Cell<bool>,
    entity_index: EntityId,
}

impl AddStvEntity {
    pub fn new(entity_index: EntityId) -> AddStvEntity {
        AddStvEntity {
            added: Cell::new(false),
            entity_index
        }
    }
}

impl MessageMutator for AddStvEntity {
    fn mutate_message(&self, message: &mut Message) {
        if !self.added.get() {
            if let Message::PacketEntities(ent_message) = message {
                let player_entity = ent_message.entities.iter().find(|ent| ent.entity_index >= 1 && ent.entity_index < 255).expect("Failed to find a player entity");
                if player_entity.entity_index == self.entity_index {
                    panic!("already an stv entity?");
                }
                let server_class = player_entity.server_class;
                dbg!(server_class);

                ent_message.entities.push(PacketEntity {
                    server_class,
                    entity_index: self.entity_index,
                    baseline_props: vec![],
                    props: vec![],
                    in_pvs: false,
                    update_type: UpdateType::Enter,
                    serial_number: 1234567,
                    delay: None
                });
                ent_message.entities.sort_by(|a, b| a.entity_index.cmp(&b.entity_index));
                self.added.set(true);
            }
        }
    }
}

fn find_stv(demo: &Demo) -> Option<EntityId> {
    let parser = DemoParser::new(demo.get_stream());
    let (_, data) = parser.parse().expect("failed to parse demo");
    data.users.values().find(|user| user.steam_id == "BOT")
        .map(|user| user.entity_id)
}