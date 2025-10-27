use serde::*;
use colored::Colorize;

#[derive(Serialize, Deserialize, Debug)]
pub enum ChatMessage {
    Join(String),
    Leave(String),
    Text { author: String, message: String },
    Whisper{author: String, message: String, recipient: String}
}


#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage{
    ErrorNameTaken,
    ErrorUnknownRecipient(String),
    IncomingWhisper{author: String, message: String},
    IncomingMessage{author: String, message: String},
    SuccessfullyWhispered{recipient: String, message:String},
    SuccessfullyJoined{active: Vec<String>},
    UserLeft(String),
    UserJoined(String),
    Leave

}


pub const SERVER_PORT: u16 = 5555;
pub const CLIENT_PORT: u16 = 4444;

pub fn deserialize_chat_message(buf: &[u8], length: usize) -> ChatMessage {
    let deserialized: ChatMessage = serde_json::from_str(&String::from_utf8_lossy(&buf[..length])).unwrap();
    return deserialized;
}

pub fn serialize_chat_message(msg: ChatMessage) -> String {
    let serialized = serde_json::to_string(&msg).unwrap();
    return serialized;
}

pub fn parse_chat_message(raw_message: String, username: &String) -> Option<ChatMessage> {
    if raw_message.to_lowercase().starts_with("quit"){
        return Some(ChatMessage::Leave(username.to_string()));
    }
    if raw_message.to_lowercase().starts_with("/w"){
        let split_msg: Vec<&str> = raw_message.trim().split(" ").collect();
        if split_msg.len() < 3 {
            return None
        }
        let recipient = split_msg[1].to_string();
        let message = split_msg[2..].join(" ");
        return Some(ChatMessage::Whisper{recipient: recipient, author: username.to_string(), message});
    }
    return Some(ChatMessage::Text{author: username.to_string(), message: raw_message.trim().to_string()});
}

pub fn deserialize_server_message(buf: &[u8], length: usize) -> ServerMessage {
    let deserialized: ServerMessage = serde_json::from_str(&String::from_utf8_lossy(&buf[..length])).unwrap();
    return deserialized;
}

pub fn serialize_server_message(msg: ServerMessage) -> String {
    let serialized = serde_json::to_string(&msg).unwrap();
    return serialized;
}

pub fn prettify_print(msg: ServerMessage, user: &String) {
    let server = "[SERVER]";
    match msg {
        ServerMessage::ErrorNameTaken => println!("{}: {} ",
            server.green(),
            "Error: Username is already taken.".red()
            ),
        ServerMessage::ErrorUnknownRecipient(recipient) => println!("{}: {} {}",
            server.green(),
            "Error: Unknown user:".red(),
            recipient.trim_ascii().bold().cyan()
            ),
        ServerMessage::IncomingWhisper{
            author,
            message
        } => println!("{}: {} has whispered to you: {}",
            server.green(),
            author.trim_ascii().bold().cyan(),
            message
            ),
        ServerMessage::IncomingMessage{
            author,
            message
        } => {
            if author == *user {
                println!("{}: {}: {}",
                server.green(),
                author.trim_ascii().bold().magenta(),
                message
                );
            }
            else{
                println!("{}: {}: {}",
                server.green(),
                author.trim_ascii().bold().green(),
                message
                );
            }    
        },
        ServerMessage::SuccessfullyWhispered{recipient, message} => println!("{}: {} {}: {}",
            server.green(),
            "Successfully whispered to".cyan(),
            recipient.trim_ascii().bold().cyan(),
            message
            ),
        ServerMessage::SuccessfullyJoined{active} => {
            let online_user = active.len();
            if online_user == 0 {
                println!("{}: {}",
                    server.green(),
                    "Successfully joined the chat. Right now you are the only one here.",
                );
            }
            else {
                println!("{}: {}{}",
                    server.green(),
                    "Successfully joined the chat. Active users are:\n",
                    active.join("\n").bold().green()
                );
            }
            
        },
        ServerMessage::UserLeft(username) => println!("{}: {} has left the chat.", 
            server.green(),
            username.trim_ascii().bold().red(),
            ),
        ServerMessage::UserJoined(username) => println!("{}: {} has joined the chat.", 
            server.green(),
            username.trim_ascii().bold().green(),
            ),
        ServerMessage::Leave => println!("{}: Leaving the room. Bye!", 
            "[CLIENT]".yellow()
            ),
    }
}

pub fn prettify_print_string(msg: String, is_error: bool) {
    let client = "[CLIENT]";
    if is_error {
        println!("{}: {}",
        client.green(),
        msg.red()
        )
    }
    else {
        println!("{}: {}",
        client.green(),
        msg
        )
    }
    
}
