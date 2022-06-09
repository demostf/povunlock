use wasm_bindgen::prelude::*;
use tf_demo_parser::{Demo};
use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::parser::{RawPacketStream, DemoHandler, Encode};
use tf_demo_parser::demo::packet::{Packet, PacketType};
use tf_demo_parser::demo::message::Message;
use bitbuffer::{BitWriteStream, LittleEndian, BitRead, BitWrite};
use tf_demo_parser::demo::message::packetentities::PacketEntitiesMessage;
use tf_demo_parser::demo::sendprop::{SendProp, SendPropIdentifier, SendPropValue};

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

const LOCAL_PROP: SendPropIdentifier = SendPropIdentifier::new("DT_LocalPlayerExclusive", "m_nTickBase");
const EYE_X_PROP: SendPropIdentifier = SendPropIdentifier::new("DT_TFLocalPlayerExclusive", "m_angEyeAngles[0]");
const EYE_Y_PROP: SendPropIdentifier = SendPropIdentifier::new("DT_TFLocalPlayerExclusive", "m_angEyeAngles[1]");

#[wasm_bindgen]
pub fn unlock(input: &[u8]) -> Vec<u8> {
    set_panic_hook();
    let mut out_buffer = Vec::with_capacity(input.len());
    {
        let mut out_stream = BitWriteStream::new(&mut out_buffer, LittleEndian);

        let demo = Demo::new(&input);
        let mut stream = demo.get_stream();
        let header = Header::read(&mut stream).unwrap();
        header.write(&mut out_stream).unwrap();

        let mut packets = RawPacketStream::new(stream.clone());
        let mut handler = DemoHandler::default();
        handler.handle_header(&header);

        let mut local_player_entity_id = None;

        while let Some(mut packet) = packets.next(&handler.state_handler).unwrap() {
            match &mut packet {
                Packet::Signon(message_packet) | Packet::Message(message_packet) => {
                    for msg in &mut message_packet
                        .messages {
                        if let Message::PacketEntities(PacketEntitiesMessage { entities, .. }) = msg {
                            for entity in entities {
                                if local_player_entity_id.is_none() && entity.get_prop_by_identifier(&LOCAL_PROP).is_some() {
                                    dbg!(entity.entity_index);
                                    local_player_entity_id = Some(entity.entity_index);
                                }

                                if Some(entity.entity_index) == local_player_entity_id {
                                    let index_x = handler.state_handler.index_for_prop(entity.server_class, EYE_X_PROP).expect("index_x not found");
                                    let index_y = handler.state_handler.index_for_prop(entity.server_class, EYE_Y_PROP).expect("index_y not found");

                                    entity.apply_update(&[
                                        SendProp {
                                            index: index_x,
                                            identifier: EYE_X_PROP,
                                            value: SendPropValue::Float(message_packet.meta.view_angles[0].local_angles.x),
                                        },
                                        SendProp {
                                            index: index_y,
                                            identifier: EYE_Y_PROP,
                                            value: SendPropValue::Float(message_packet.meta.view_angles[0].local_angles.y),
                                        },
                                    ])
                                }
                            }
                        }
                    }

                    message_packet.meta.view_angles = Default::default();
                    message_packet
                        .messages
                        .iter_mut()
                        .for_each(|msg| if let Message::ServerInfo(info) = msg {
                            info.stv = true;
                        });
                }
                _ => {}
            }

            if packet.packet_type() != PacketType::ConsoleCmd && packet.packet_type() != PacketType::UserCmd {
                packet
                    .encode(&mut out_stream, &handler.state_handler)
                    .unwrap();
            }
            handler.handle_packet(packet).unwrap();
        }
    }
    out_buffer
}