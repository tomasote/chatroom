use shared::*;
use std::io::{self, BufRead, Write, Read};
use std::net::TcpStream;
use std::thread;

fn main() {
    println!("Enter your username for the chatroom: ");
    let mut username = String::new();
    let _ = io::stdin().read_line(&mut username);
    let msg = ChatMessage::Join(String::from(username.trim()));
    let mut stream = TcpStream::connect(format!("192.168.0.163:{}",SERVER_PORT)).expect("Could not connect to server");
    stream.write_all(serialize_chat_message(msg).as_bytes()).unwrap();
    println!("Connected to server as {}. Type messages and press Enter to send.", username);
    let user = username.clone();
    let username_for_handler = username.clone();
    println!("Type 'quit' to exit.");
    let mut read_stream = stream.try_clone().expect("Failed to clone stream");
    let mut handler_stream = stream.try_clone().expect("Failed to clone stream");
    let _ = ctrlc::set_handler(move || {
        let msg = ChatMessage::Leave(String::from(username_for_handler.trim()));
        handler_stream.write_all(serialize_chat_message(msg).as_bytes()).unwrap();
    });
    // Clone the stream handle for the reading thread

    // Spawn a thread to continuously listen for server messages
    thread::spawn(move || {
        let mut buffer = [0u8; 512];
        loop {
            match read_stream.read(&mut buffer) {
                Ok(0) => {
                    prettify_print_string("Server disconnected.".to_string(), false);
                    std::process::exit(0);
                }
                Ok(n) => {
                    let user_ref = &user;
                    let msg = deserialize_server_message(&buffer, n);
                    match msg{
                        ServerMessage::ErrorUnknownRecipient(_) => prettify_print(msg, user_ref),
                        ServerMessage::IncomingWhisper{author: _, message: _} => prettify_print(msg, user_ref),
                        ServerMessage::IncomingMessage{author: _, message: _} => prettify_print(msg, user_ref),
                        ServerMessage::SuccessfullyWhispered{recipient: _} => prettify_print(msg, user_ref),
                        ServerMessage::SuccessfullyJoined{active: _} => prettify_print(msg, user_ref),
                        ServerMessage::Leave => {
                            prettify_print(msg, user_ref);
                            std::process::exit(0);
                        },
                        ServerMessage::ErrorNameTaken =>{
                            prettify_print(msg, user_ref);
                            std::process::exit(0);
                        },
                        ServerMessage::UserJoined(_) => prettify_print(msg, user_ref),
                        ServerMessage::UserLeft(ref left_user) => {
                            if *left_user == *user_ref {
                                std::process::exit(0);
                            }
                            prettify_print(msg, user_ref);
                        }
                    }
                    io::stdout().flush().unwrap();
                }
                Err(e) => {
                    prettify_print_string(format!("Error reading from server: {}", e), true);
                    std::process::exit(0);
                }
            }
        }
    });

    // Main thread: read user input and send to server
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let msg_raw = line.unwrap();
        match parse_chat_message(msg_raw, &username) {
            None => prettify_print_string(String::from("Improperly formatted message. Not sending."), true),
            Some(msg) => stream.write_all(serialize_chat_message(msg).as_bytes()).unwrap()
        }
        
    }
}

