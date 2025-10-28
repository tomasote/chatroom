use shared::*;
use std::io::{Write, Read};
use std::sync::{Arc, Mutex};
use std::net::TcpStream;
use std::thread;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicBool;

fn main() -> eframe::Result{

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    let name = Arc::new(Mutex::new(String::from("Tomas")));
    let messages = Arc::new(Mutex::new(Vec::<String>::new()));
    
    let active_users = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut stream = Arc::new(Mutex::new(None::<TcpStream>));
    let mut username_ok = true;
    let error = Arc::new(Mutex::new(String::from("")));
    let is_joined = Arc::new(Mutex::new(false));

    let repaint_flag = Arc::new(AtomicBool::new(false));


    let mut chat_message = String::from("");
    
    let userctrl = name.clone();
    let streamctrl = stream.clone();
    let is_joinedctrl = is_joined.clone();
    let _ = ctrlc::set_handler(move || {
        if *is_joinedctrl.lock().unwrap() {
            let msg = ChatMessage::Leave(userctrl.lock().unwrap().trim().to_string());
            let mut stream_lock = streamctrl.lock().unwrap();
            if let Some(ref mut s) = *stream_lock {
                s.write_all(serialize_chat_message(msg).as_bytes()).unwrap();
            }
        }
        std::process::exit(0);
    });
   
    eframe::run_simple_native("Chatroom", options, move |ctx, frame| {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        if repaint_flag.load(Ordering::Relaxed){
            println!("Request noticed");
                ctx.request_repaint();
                repaint_flag.store(false, Ordering::Relaxed);
            }
        let mut name_lock = name.lock().unwrap();
        let logged_in_lock = is_joined.lock().unwrap();
        let mut error_lock = error.lock().unwrap();
        if !*logged_in_lock{
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Join the chatroom");
                ui.horizontal(|ui| {
                    let name_label = ui.label("Enter your username:");
                    ui.text_edit_singleline(&mut *name_lock)
                        .labelled_by(name_label.id);
                });
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Join").clicked() || enter_pressed{
                    let mut stream_lock = stream.lock().unwrap();
                    match TcpStream::connect(format!("127.0.0.1:{}", SERVER_PORT)) {
                        Ok(new_stream) => {
                            let read_stream = new_stream.try_clone().unwrap();
                            *stream_lock = Some(new_stream);
                            let name_val = name_lock.clone();
                            username_ok = handle_login(&mut stream_lock, &name_val);
                            spawn_listener(read_stream, name.clone(), messages.clone(), active_users.clone(), is_joined.clone(), error.clone(), Arc::clone(&repaint_flag));
                        },
                        Err(e) => {
                            *error_lock = e.to_string();
                        }
                    }
                    //username_ok = handleLogin(&mut stream, &name_lock);
                }
                if !username_ok{
                    ui.label("Username can not be empty.");
                }
                if *error_lock != ""{
                    ui.label(&*error_lock);
                } 
            });
        }
        else{
            
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Chatroom");
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.label("Messages");
                    ui.add_space(470.0);
                    ui.label("Active users");
                });
                ui.horizontal(|ui| {
                    egui::Frame::group(ui.style())
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY)) // border thickness + color
                    .show( ui, |ui| {
                        ui.set_min_size(egui::vec2(500.0, 400.0));
                        egui::ScrollArea::vertical()
                            .id_salt("messages")
                            .show(ui, |ui| {
                                ui.set_width(500.0);
                                ui.vertical(|ui| {
                                    let msgs = messages.lock().unwrap();
                                    for msg in msgs.iter() {
                                        ui.label(msg);
                                    }
                                });
                                })
                    });
                    ui.add_space(10.0);
                    egui::Frame::group(ui.style())
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY)) // border thickness + color
                    .show( ui, |ui| {
                        ui.set_min_size(egui::vec2(230.0, 400.0));
                        egui::ScrollArea::vertical()
                            .id_salt("users")
                            .show( ui, |ui| {
                                ui.set_width(230.0);
                                ui.vertical(|ui| {
                                    let users = active_users.lock().unwrap();
                                    for user in users.iter() {
                                        ui.label(user);
                                    }
                                });
                                
                            });
                    });


                });
                
                ui.add_space(10.0);
                // Input field chat
                ui.horizontal(|ui| {
                            let name_label = ui.label("Message:");
                            egui::Frame::group(ui.style())
                                .fill(ui.visuals().extreme_bg_color)
                                .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY))
                                .corner_radius(egui::CornerRadius::same(4))
                                .show(ui, |ui| {
                                    let mut text_edit = egui::TextEdit::singleline(&mut chat_message);
                                    let resp = ui.add(text_edit);
                                    resp.request_focus();
                                }); 
                            });
                
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                ui.horizontal(|ui| {
                    // Send button
                    if ui.button("Send").clicked() || enter_pressed {
                        handle_send(&mut stream, &mut chat_message, &name_lock);
                        chat_message = String::from("");
                    }

                    // Leave button 
                    if ui.button("Leave").clicked() || esc_pressed{
                        handle_leave(&mut stream, &name_lock);
                    }
                });
            });
        }
        
        
    })

}

fn handle_login(stream: &mut Option<TcpStream>, name : &String) -> bool{
    if name.trim() == ""{
        return false;
    }
    let msg = ChatMessage::Join(String::from(name.trim()));
    if let Some(s) = stream.as_mut() {
        let _ = s.write_all(serialize_chat_message(msg).as_bytes());
    }
    return true;
}

fn handle_send(stream: &Arc<Mutex<Option<TcpStream>>>, message: &str, user: &str) {
    if message.trim() == "" {return;}
    let parsed = parse_chat_message(message.to_string(), &user.to_string());
    if let Some(msg) = parsed {
        if let Some(ref mut s) = *stream.lock().unwrap() {
            let _ = s.write_all(serialize_chat_message(msg).as_bytes());
        }
    }
}

fn handle_leave(stream: &Arc<Mutex<Option<TcpStream>>>, user: &str) {
    if let Some(ref mut s) = *stream.lock().unwrap() {
        let msg = ChatMessage::Leave(user.trim().to_string());
        let _ = s.write_all(serialize_chat_message(msg).as_bytes());
    }
}

fn spawn_listener(
    read_stream: TcpStream,
    name: Arc<Mutex<String>>,
    messages: Arc<Mutex<Vec<String>>>,
    active_users: Arc<Mutex<Vec<String>>>,
    is_joined: Arc<Mutex<bool>>,
    error: Arc<Mutex<String>>,
    repaint_flag: Arc<AtomicBool>
) {
    thread::spawn(move || {
        let mut read_stream = read_stream;
        let mut buffer = [0u8; 512];
        loop {
            match read_stream.read(&mut buffer) {
                Ok(0) => {
                    let mut is_joined_ref = is_joined.lock().unwrap();
                    *is_joined_ref = false;
                    break;
                }
                Ok(n) => {
                    let user_ref = name.lock().unwrap();
                    let msg = deserialize_server_message(&buffer, n);
                    match msg{
                        ServerMessage::ErrorUnknownRecipient(_) => prettify_print(msg, &user_ref),
                        ServerMessage::IncomingWhisper{ref author, ref message} => {
                            let formatted = format!("[SERVER]: {} => you: {}", author, message);
                            let mut messages_ref = messages.lock().unwrap();
                            messages_ref.push(formatted);
                            prettify_print(msg, &user_ref);
                        },
                        ServerMessage::IncomingMessage{author, message} => {
                            //prettify_print(msg, &user_ref);
                            let formatted = format!("{}: {}", author, message);
                            let mut messages_ref = messages.lock().unwrap();
                            messages_ref.push(formatted);
                        },
                        ServerMessage::SuccessfullyWhispered{ref recipient, ref message} =>{
                            let formatted = format!("[SERVER]: You => {}: {}", recipient, message);
                            let mut messages_ref = messages.lock().unwrap();
                            messages_ref.push(formatted);
                            prettify_print(msg, &user_ref)
                        }, 
                        ServerMessage::SuccessfullyJoined{ref active} => {
                            let mut is_joined_ref = is_joined.lock().unwrap();
                            *is_joined_ref = true;
                            let mut error_lock = error.lock().unwrap();
                            *error_lock = "".to_string();
                            let mut active_users_lock = active_users.lock().unwrap();
                            *active_users_lock = active.to_vec();
                            prettify_print(msg, &user_ref);

                        }
                        ServerMessage::Leave => {
                            prettify_print(msg, &user_ref);
                            let mut is_joined_ref = is_joined.lock().unwrap();
                            *is_joined_ref = false;

                        },
                        ServerMessage::ErrorNameTaken =>{
                            prettify_print(msg, &user_ref);
                            let mut error_lock = error.lock().unwrap();
                            *error_lock = "Username is already taken.".to_string();
                            let mut is_joined_ref = is_joined.lock().unwrap();
                            *is_joined_ref = false;
                            //std::process::exit(0);
                        },
                        ServerMessage::UserJoined(ref joined_user) => {
                            if *joined_user != *user_ref {
                                let formatted = format!("[SERVER]: {} has joined the chat.", *joined_user);
                                let mut messages_ref = messages.lock().unwrap();
                                messages_ref.push(formatted);
                                let mut active_users_ref = active_users.lock().unwrap();
                                if active_users_ref.iter().all(|i| *i != *joined_user) {
                                    active_users_ref.push(joined_user.clone())
                                }
                            }
                            prettify_print(msg, &user_ref);

                        },
                        ServerMessage::UserLeft(ref left_user) => {
                            if *left_user != *user_ref {
                                let formatted = format!("[SERVER]: {} has left the chat.", *left_user);
                                let mut messages_ref = messages.lock().unwrap();
                                messages_ref.push(formatted);
                                let mut active_users_ref = active_users.lock().unwrap();
                                let index = active_users_ref.iter().position(|x| *x == *left_user);
                                match index {
                                    Some(idx) => active_users_ref.remove(idx),
                                    None => todo!(),
                                };
                            }
                            prettify_print(msg, &user_ref);

                        }
                    }
                    repaint_flag.store(true, Ordering::Relaxed);
                }
                Err(_) => break,
            }
        }
    });
}