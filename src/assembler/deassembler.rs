#![allow(unused)]
use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::{fs, thread};
use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, FloodResponse, Fragment, NodeType};
use wg_2024::packet::{Packet, PacketType};
use rand::*;



pub const FRAGMENT_DSIZE: usize = 128;


pub struct Deassembler{
    //mappa i messaggi in base ad un id
    received_message:HashMap<u64, Vec<u8>>
}



impl Deassembler {
    //create hashmap empty
    pub fn new()->Self {
        Deassembler{
            received_message:HashMap::new()
        }
    }

    //adds a message to the list, fragments it and returns the fragments
    pub fn add_message(&mut self, message:Vec<u8>, session_id:u64)->Vec<Fragment> {
        //add message to hasmap and call deassemble
        self.received_message.insert(session_id, message);
        return self.deassemble_message(session_id);
    }



    pub fn deassemble_message(&mut self, session_id:u64)->Vec<Fragment> {
        if let Some(mut message) = self.received_message.remove(&session_id) {
            
            let mut fragments=Vec::<Fragment>::new();
            //ceil rounds the decimal number to the next whole number
            let total_fragments = (message.len() as f64 / FRAGMENT_DSIZE as f64).ceil() as u64;

            // Break the message into fragments (chunks) and iter over it
            for (i, chunk) in message.chunks(FRAGMENT_DSIZE).enumerate() { //divide the message in chunks
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
        } else {
            Vec::new() //to fix
        }
    }
}