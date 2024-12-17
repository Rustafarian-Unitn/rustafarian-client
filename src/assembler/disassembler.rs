#![allow(unused)]
use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};
use rand::*;
use std::collections::{HashMap, HashSet};
use std::{fs, thread};
use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, FloodResponse, Fragment, NodeType};
use wg_2024::packet::{Packet, PacketType};

pub const FRAGMENT_DSIZE: usize = 128;

pub struct Disassembler {}

impl Disassembler {
    //create hashmap empty
    pub fn new() -> Self {
        Disassembler {}
    }

    pub fn disassemble_message(&mut self, message: Vec<u8>, session_id: u64) -> Vec<Fragment> {
        let mut fragments = Vec::<Fragment>::new();
        //ceil rounds the decimal number to the next whole number
        let total_fragments = (message.len() as f64 / FRAGMENT_DSIZE as f64).ceil() as u64;

        // Break the message into fragments (chunks) and iter over it
        for (i, chunk) in message.chunks(FRAGMENT_DSIZE).enumerate() {
            //divide the message in chunks
            let mut data = [0u8; FRAGMENT_DSIZE];
            let length = chunk.len() as u8; // chunk length

            // copy chucnk into data
            data[..length as usize].copy_from_slice(chunk);

            // create fragment and add it to list
            let fragment = Fragment {
                fragment_index: i as u64,
                total_n_fragments: total_fragments,
                length,
                data,
            };

            fragments.push(fragment);
        }

        fragments
    }
}
