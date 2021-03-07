use mio::{Interest, Token};
use std::{collections::BTreeMap, time::Duration};
use std::{
    io::{ErrorKind::WouldBlock, Read},
    net::SocketAddr,
};

pub struct Source(pub usize);

#[derive(Debug)]
pub enum NetEvent<'a> {
    Accepted(SocketAddr),
    Disconnected,
    Received(&'a [u8]),
    Tick,
    // Event,
}

pub struct NetServer {
    mio_poll: mio::Poll,
    mio_events: mio::Events,
    ready_sockets: Vec<Token>,
    listener: mio::net::TcpListener,
    sockets: BTreeMap<Token, mio::net::TcpStream>,
    tick_duration: Option<Duration>,
    last_token: usize,
    read_buffer: Box<[u8; 4096]>,
}

impl NetServer {
    pub fn new(addr: SocketAddr) -> Self {
        let mio_poll = mio::Poll::new().unwrap();

        let mut listener = mio::net::TcpListener::bind(addr).unwrap();

        mio_poll
            .registry()
            .register(&mut listener, Token(1), Interest::READABLE)
            .unwrap();

        NetServer {
            mio_poll,
            mio_events: mio::Events::with_capacity(128),
            ready_sockets: Vec::with_capacity(128),
            listener,
            sockets: BTreeMap::new(),
            tick_duration: None,
            last_token: 2,
            read_buffer: Box::new([0; 4096]),
        }
    }

    pub fn set_tick(&mut self, tick_duration: Duration) {
        self.tick_duration = Some(tick_duration);
    }

    pub fn receive_event(&mut self) -> (Source, NetEvent<'_>) {
        // TODO: Check interrupt
        loop {
            match self.ready_sockets.first() {
                Some(&token) => {
                    if token == Token(1) {
                        match self.listener.accept() {
                            Ok((mut tcp_stream, socket_addr)) => {
                                let new_token = Token(self.last_token);
                                self.last_token += 1;
                                self.mio_poll
                                    .registry()
                                    .register(&mut tcp_stream, new_token, Interest::READABLE)
                                    .unwrap();
                                self.sockets.insert(new_token, tcp_stream);
                                self.last_token += 1;
                                break (Source(token.0), NetEvent::Accepted(socket_addr));
                            }
                            Err(e) if e.kind() == WouldBlock => {
                                self.ready_sockets.pop();
                                continue;
                            }
                            Err(_e) => panic!("Sad..."),
                        }
                    } else {
                        let stream = self.sockets.get_mut(&token).expect("Unregistered token");

                        match stream.read(self.read_buffer.as_mut()) {
                            Ok(bytes) if bytes == 0 => {
                                self.mio_poll.registry().deregister(stream).unwrap();
                                self.sockets
                                    .remove(&token)
                                    .expect("Was looking at it a second ago");
                                self.ready_sockets.retain(|t| *t != token);
                                break (Source(token.0), NetEvent::Disconnected);
                            }
                            Ok(bytes) => {
                                break (
                                    Source(token.0),
                                    NetEvent::Received(&self.read_buffer[..bytes]),
                                );
                            }
                            Err(e) if e.kind() == WouldBlock => {
                                self.ready_sockets.pop();
                                continue;
                            }
                            Err(_e) => panic!("So sad..."),
                        }
                    }
                }
                None => {
                    self.mio_poll
                        .poll(&mut self.mio_events, self.tick_duration)
                        .unwrap();
                    if self.mio_events.is_empty() {
                        break (Source(0), NetEvent::Tick);
                    } else {
                        self.ready_sockets
                            .extend(self.mio_events.iter().map(|event| event.token()));
                        continue;
                    }
                }
            }
        }
    }
}
