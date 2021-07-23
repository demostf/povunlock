mod utils;

use wasm_bindgen::prelude::*;
use tf_demo_parser::{Demo, MessageType};
use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::parser::{RawPacketStream, DemoHandler, NullHandler, Encode};
use tf_demo_parser::demo::packet::{Packet, PacketType};
use tf_demo_parser::demo::message::Message;
use std::error::Error;
use bitbuffer::{BitWriteStream, LittleEndian, BitRead, BitWrite};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn unlock(input: &[u8]) -> Vec<u8> {
    let mut out_buffer = Vec::with_capacity(input.len());
    {
        let mut out_stream = BitWriteStream::new(&mut out_buffer, LittleEndian);

        let demo = Demo::new(&input);
        let mut stream = demo.get_stream();
        let header = Header::read(&mut stream).unwrap();
        header.write(&mut out_stream).unwrap();

        let mut packets = RawPacketStream::new(stream.clone());
        let mut handler = DemoHandler::parse_all_with_analyser(NullHandler);
        handler.handle_header(&header);

        while let Some(mut packet) = packets.next(&handler.state_handler).unwrap() {
            match &mut packet {
                Packet::Sigon(message_packet) | Packet::Message(message_packet) => {
                    message_packet.meta.view_angles = Default::default();
                    let messages = std::mem::take(&mut message_packet.messages);
                    let messages = messages
                        .into_iter()
                        .filter(|msg| msg.get_message_type() != MessageType::SetView)
                        .map(|mut msg| {
                            match &mut msg {
                                Message::ServerInfo(info) => {
                                    info.stv = true;
                                }
                                _ => {}
                            };
                            msg
                        })
                        .collect::<Vec<_>>();
                    message_packet.messages = messages;
                }
                _ => {}
            }

            if packet.packet_type() != PacketType::ConsoleCmd {
                packet
                    .encode(&mut out_stream, &handler.state_handler)
                    .unwrap();
            }
            handler.handle_packet(packet).unwrap();
        }
    }
    out_buffer
}