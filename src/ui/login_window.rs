//! greetd owns PAM, privileges, PAM environment and session creation.
//! The shell never runs as root and never compares passwords itself.
use super::{button, status, Buttons};
use anyhow::{bail, Result};
use gtk::prelude::*;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    io::{Read, Write},
    os::unix::net::UnixStream,
    rc::Rc,
};
struct Greeter {
    stream: UnixStream,
    active: bool,
}
impl Greeter {
    fn request(&mut self, value: Value) -> Result<Value> {
        let bytes = serde_json::to_vec(&value)?;
        self.stream.write_all(&(bytes.len() as u32).to_ne_bytes())?;
        self.stream.write_all(&bytes)?;
        let mut len = [0; 4];
        self.stream.read_exact(&mut len)?;
        let len = u32::from_ne_bytes(len) as usize;
        if len > 1024 * 1024 {
            bail!("Invalid greetd response size");
        }
        let mut bytes = vec![0; len];
        self.stream.read_exact(&mut bytes)?;
        let response: Value = serde_json::from_slice(&bytes)?;
        if response["type"] == "error" {
            self.active = false;
            bail!("Authentication or session request rejected");
        }
        Ok(response)
    }
    fn begin(&mut self, user: &str) -> Result<String> {
        if self.active {
            self.request(json!({"type":"cancel_session"}))?;
        }
        let r = self.request(json!({"type":"create_session", "username":user}))?;
        self.active = true;
        self.handle(r)
    }
    fn respond(&mut self, password: &str) -> Result<String> {
        if !self.active {
            bail!("Choose a user first");
        }
        let r = self.request(json!({"type":"post_auth_message_response", "response":password}))?;
        self.handle(r)
    }
    fn handle(&mut self, mut response: Value) -> Result<String> {
        for _ in 0..16 {
            match response["type"].as_str() {
                Some("success") => {
                    self.request(json!({"type":"start_session", "cmd":["/usr/local/bin/tuxconsole-session"], "env":[]}))?;
                    self.active = false;
                    return Ok("Session started".into());
                }
                Some("auth_message") => match response["auth_message_type"].as_str() {
                    Some("secret") | Some("visible") => {
                        return Ok(response["auth_message"]
                            .as_str()
                            .unwrap_or("Password")
                            .into())
                    }
                    _ => {
                        response = self.request(
                            json!({"type":"post_auth_message_response", "response":null}),
                        )?
                    }
                },
                _ => bail!("Unexpected greetd response"),
            }
        }
        bail!("Too many authentication messages")
    }
}
pub fn build(buttons: &Buttons, note: &gtk::Label) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let connection = std::env::var_os("GREETD_SOCK").and_then(|p| UnixStream::connect(p).ok());
    let Some(stream) = connection else {
        note.set_text("Cannot connect to greetd");
        return root;
    };
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(10)));
    let greeter = Rc::new(RefCell::new(Greeter {
        stream,
        active: false,
    }));
    let password = gtk::PasswordEntry::new();
    password.set_show_peek_icon(false);
    root.append(&password);
    if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
        for line in passwd.lines() {
            let f: Vec<_> = line.split(':').collect();
            if f.len() != 7
                || !f[2]
                    .parse::<u32>()
                    .is_ok_and(|uid| (1000..65534).contains(&uid))
                || f[6].ends_with("nologin")
                || f[6].ends_with("false")
            {
                continue;
            }
            let user = f[0].to_string();
            let g = greeter.clone();
            let n = note.clone();
            let p = password.clone();
            let title = format!("Sign in: {user}");
            let selector = button(&title, buttons, move || {
                p.set_text("");
                p.grab_focus();
                status(&n, g.borrow_mut().begin(&user));
            });
            let face = std::path::PathBuf::from(f[5]).join(".face");
            let avatar = if face.is_file() {
                gtk::Image::from_file(face)
            } else {
                gtk::Image::from_icon_name("avatar-default-symbolic")
            };
            avatar.set_pixel_size(48);
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.append(&avatar);
            row.append(&gtk::Label::new(Some(&title)));
            selector.set_child(Some(&row));
            root.append(&selector);
        }
    }
    let grid = gtk::Grid::new();
    for (i, ch) in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 !@#$%^&*()-_=+[]{};:'\",.<>/?\\|`~".chars().enumerate() {
        let p = password.clone(); grid.attach(&button(&ch.to_string(), buttons, move || { let mut text = p.text().to_string(); text.push(ch); p.set_text(&text); }), (i % 12) as i32, (i / 12) as i32, 1, 1);
    }
    let mut key = grid.first_child();
    while let Some(widget) = key {
        widget.set_size_request(64, 48);
        key = widget.next_sibling();
    }
    let p = password.clone();
    root.append(&button("Backspace", buttons, move || {
        let mut text = p.text().to_string();
        text.pop();
        p.set_text(&text);
    }));
    let n = note.clone();
    root.append(&button("Authenticate", buttons, move || {
        let secret = password.text();
        password.set_text("");
        status(&n, greeter.borrow_mut().respond(secret.as_str()));
    }));
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(100)
        .vexpand(true)
        .child(&grid)
        .build();
    root.append(&scroll);
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(s: &mut UnixStream) -> Value {
        let mut len = [0; 4];
        s.read_exact(&mut len).unwrap();
        let mut data = vec![0; u32::from_ne_bytes(len) as usize];
        s.read_exact(&mut data).unwrap();
        serde_json::from_slice(&data).unwrap()
    }
    fn send(s: &mut UnixStream, v: Value) {
        let bytes = serde_json::to_vec(&v).unwrap();
        s.write_all(&(bytes.len() as u32).to_ne_bytes()).unwrap();
        s.write_all(&bytes).unwrap();
    }
    #[test]
    fn pam_conversation_starts_session_only_after_success() {
        let (stream, mut server) = UnixStream::pair().unwrap();
        let t = std::thread::spawn(move || {
            assert_eq!(
                read(&mut server),
                json!({"type":"create_session", "username":"player"})
            );
            send(
                &mut server,
                json!({"type":"auth_message", "auth_message_type":"secret", "auth_message":"Password:"}),
            );
            assert_eq!(read(&mut server)["type"], "post_auth_message_response");
            send(&mut server, json!({"type":"success"}));
            let request = read(&mut server);
            assert_eq!(request["type"], "start_session");
            assert_eq!(request["cmd"], json!(["/usr/local/bin/tuxconsole-session"]));
            send(&mut server, json!({"type":"success"}));
        });
        let mut g = Greeter {
            stream,
            active: false,
        };
        assert_eq!(g.begin("player").unwrap(), "Password:");
        assert_eq!(g.respond("test-only").unwrap(), "Session started");
        t.join().unwrap();
    }
    #[test]
    fn rejection_does_not_start_session() {
        let (stream, mut server) = UnixStream::pair().unwrap();
        let t = std::thread::spawn(move || {
            read(&mut server);
            send(
                &mut server,
                json!({"type":"error", "error_type":"auth_error"}),
            );
        });
        let mut g = Greeter {
            stream,
            active: false,
        };
        assert!(g.begin("player").is_err());
        assert!(!g.active);
        t.join().unwrap();
    }
    #[test]
    fn rejects_oversized_frames() {
        let (stream, mut server) = UnixStream::pair().unwrap();
        let t = std::thread::spawn(move || {
            read(&mut server);
            server.write_all(&u32::MAX.to_ne_bytes()).unwrap();
        });
        let mut g = Greeter {
            stream,
            active: false,
        };
        assert!(g.begin("player").is_err());
        t.join().unwrap();
    }
}
