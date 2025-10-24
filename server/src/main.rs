use shared::*;
use std::io::{Read, Write};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::{Arc, Mutex, mpsc};

type Tx = mpsc::Sender<String>;

fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<HashMap<String, Tx>>>) {
    let peer = stream.peer_addr().unwrap();
    println!("Client connected: {}", peer);

    let mut buffer = [0u8; 512];
    let n = stream.read(&mut buffer).expect("Failed to read username");
    let deserialized: ChatMessage = deserialize_chat_message(&buffer, n);
    // We know that this will be the join message.
    let mut username = String::new();
    if let ChatMessage::Join(name) = deserialized {
        username = name;
    }
    if clients.lock().unwrap().contains_key(&username) {
        let error: ServerMessage = ServerMessage::ErrorNameTaken;
        stream.write_all(serialize_server_message(error).as_bytes()).unwrap();
        return;
    }

    else {
        let active_users = clients.lock().unwrap().clone().into_keys().collect();
        let message: ServerMessage = ServerMessage::SuccessfullyJoined{active: active_users};
        stream.write_all(serialize_server_message(message).as_bytes()).unwrap();
        let message: ServerMessage = ServerMessage::UserJoined(username.clone());
        send_to_all_users(message, &clients);
    }

    let (tx, rx) = mpsc::channel::<String>();
    {
        clients.lock().unwrap().insert(username.clone(), tx);        
    }

    let mut write_stream = stream.try_clone().unwrap();
    thread::spawn(move || {
        for msg in rx {
            if write_stream.write_all(msg.as_bytes()).is_err() {
                break; // Stop if client disconnects
            }
        }
    });
    let mut buffer = [0u8; 512];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("Client {} disconnected", peer);
                clients.lock().unwrap().remove(&username);
                break;
            }
            Ok(n) => {
                match deserialize_chat_message(&buffer, n)
                {
                    ChatMessage::Leave(user) => {
                        let message: ServerMessage = ServerMessage::UserLeft(user.clone());
                        clients.lock().unwrap().remove(&user);
                        send_to_all_users(message, &clients);
                        let response: ServerMessage = ServerMessage::Leave;
                        stream.write_all(serialize_server_message(response).as_bytes()).unwrap();
                    },
                    ChatMessage::Text{author, message} => {
                        let message: ServerMessage = ServerMessage::IncomingMessage{author, message};
                        send_to_all_users(message, &clients);
                    }
                    ChatMessage::Whisper{author, message, recipient} => {
                        let message = ServerMessage::IncomingWhisper{author, message};
                        let success = send_to_one_user(message, &clients, recipient.clone());
                        if success{
                            let response: ServerMessage = ServerMessage::SuccessfullyWhispered{recipient};
                            stream.write_all(serialize_server_message(response).as_bytes()).unwrap();
                        }
                        else {
                            let response: ServerMessage = ServerMessage::ErrorUnknownRecipient(recipient);
                            stream.write_all(serialize_server_message(response).as_bytes()).unwrap();
                        }
                    },
                    ChatMessage::Join(_user) => todo!() // should not happen
                }
            }
            Err(e) => {
                eprintln!("Error with {}: {}", peer, e);
                break;
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", SERVER_PORT)).unwrap();
    println!("Server listening on 127.0.0.1:{}", SERVER_PORT);
    let clients = Arc::new(Mutex::new(HashMap::<String, Tx>::new()));
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let clients_ref = Arc::clone(&clients);
            thread::spawn(move || handle_client(stream, clients_ref));
        }
    }
}


fn send_to_all_users(msg: ServerMessage, clients: &Arc<Mutex<HashMap<String, Tx>>>)
{
    let value = serialize_server_message(msg);
    for (_name, tx) in clients.lock().unwrap().iter() {
        let _ = tx.send(value.clone());
    }
} 

fn send_to_one_user(msg: ServerMessage, clients: &Arc<Mutex<HashMap<String, Tx>>>, username: String) -> bool
{
    match clients.lock().unwrap().get(&username){
        Some(tx) => {
            let value = serialize_server_message(msg);
            let _ = tx.send(value);
            return true;
        }
        None => return false
    }
}
