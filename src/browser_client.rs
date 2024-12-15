

// pub struct BrowserClient {
//     client_id: u8,
//     senders: HashMap<u8, Sender<Packet>>,
//     receiver: Receiver<Packet>,
// }

// impl BrowserClient {
//     pub fn new(
//         client_id: u8,
//         senders: HashMap<u8, Sender<Packet>>,
//         receiver: Receiver<Packet>,
//     ) -> Self {
//         BrowserClient {
//             client_id,
//             senders,
//             receiver,
//         }
//     }
// }

// impl Client for BrowserClient {
//     type RequestType = ServerTypeRequest;
//     type ResponseType = ServerTypeResponse;

//     fn client_id(&self) -> u8 {
//         self.client_id
//     }

//     fn senders(&self) -> &HashMap<u8, Sender<Packet>> {
//         &self.senders
//     }

//     fn receiver(&self) -> &Receiver<Packet> {
//         &self.receiver
//     }

//     fn send_response(&mut self, _response: Self::ResponseType) {
//         // send response
//     }

//     fn handle_request(&mut self, request: Self::RequestType) -> Self::ResponseType {
//         // handle request
//     }
// }
